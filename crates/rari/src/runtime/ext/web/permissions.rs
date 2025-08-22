#![allow(unused)]
use deno_fs::FsError;
use deno_permissions::PermissionCheckError;
use parking_lot::RwLock;
use rustc_hash::FxHashSet;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use deno_permissions::PermissionDeniedError;

fn to_io_err(err: PermissionDeniedError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, err.to_string())
}

pub fn oops<T>(msg: impl std::fmt::Display) -> Result<T, PermissionDeniedError> {
    use deno_permissions::PermissionDeniedError;
    Err(PermissionDeniedError { access: msg.to_string(), name: "oops" })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWebPermissions;
impl WebPermissions for DefaultWebPermissions {
    fn allow_hrtime(&self) -> bool {
        true
    }

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        Some(path)
    }

    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError> {
        Ok(p)
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError> {
        Ok(p)
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDeniedError> {
        Ok(PathBuf::from(path))
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionDeniedError> {
        Ok(())
    }

    fn check_exec(&self) -> Result<(), PermissionDeniedError> {
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
struct AllowlistWebPermissionsSet {
    pub hrtime: bool,
    pub exec: bool,
    pub read_all: bool,
    pub write_all: bool,
    pub url: FxHashSet<String>,
    pub openr_paths: FxHashSet<String>,
    pub openw_paths: FxHashSet<String>,
    pub envs: FxHashSet<String>,
    pub sys: FxHashSet<SystemsPermissionKind>,
    pub read_paths: FxHashSet<String>,
    pub write_paths: FxHashSet<String>,
    pub hosts: FxHashSet<String>,
}

#[derive(Clone, Default, Debug)]
pub struct AllowlistWebPermissions(Arc<RwLock<AllowlistWebPermissionsSet>>);
impl AllowlistWebPermissions {
    fn borrow(&self) -> parking_lot::RwLockReadGuard<'_, AllowlistWebPermissionsSet> {
        self.0.read()
    }
}

impl WebPermissions for AllowlistWebPermissions {
    fn allow_hrtime(&self) -> bool {
        self.borrow().hrtime
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        if self.borrow().hosts.contains(host) { Ok(()) } else { oops(host)? }
    }

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        if self.borrow().url.contains(url.as_str()) { Ok(()) } else { oops(url)? }
    }

    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError> {
        let inst = self.borrow();
        if inst.read_all && p.to_str().is_some_and(|s| inst.read_paths.contains(s)) {
            Ok(p)
        } else {
            oops(p.display())?
        }
    }

    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError> {
        let inst = self.borrow();
        if inst.write_all && p.to_str().is_some_and(|s| inst.write_paths.contains(s)) {
            Ok(p)
        } else {
            oops(p.display())?
        }
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        let path_str = path.to_str()?;
        if read && !self.borrow().openr_paths.contains(path_str) {
            return None;
        }
        if write && !self.borrow().openw_paths.contains(path_str) {
            return None;
        }
        Some(path)
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDeniedError> {
        if self.borrow().read_all { Ok(()) } else { oops("read_all")? }
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        if !self.borrow().read_all {
            return oops("read_all")?;
        }
        self.check_read(Cow::Borrowed(p), Some(api_name))?;
        Ok(())
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDeniedError> {
        if self.borrow().write_all { Ok(()) } else { oops("write_all")? }
    }

    fn check_write_blind(
        &self,
        path: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        self.check_write(Cow::Borrowed(path), Some(api_name))?;
        Ok(())
    }

    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDeniedError> {
        let p = self.check_write(Cow::Borrowed(Path::new(path)), Some(api_name))?;
        Ok(p.into_owned())
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError> {
        if self.borrow().sys.contains(&kind) { Ok(()) } else { oops(kind.as_str())? }
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionDeniedError> {
        if self.borrow().envs.contains(var) { Ok(()) } else { oops(var)? }
    }

    fn check_exec(&self) -> Result<(), PermissionDeniedError> {
        if self.borrow().exec { Ok(()) } else { oops("ffi")? }
    }
}

pub trait WebPermissions: std::fmt::Debug + Send + Sync {
    fn allow_hrtime(&self) -> bool;

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError>;

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>>;

    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError>;

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDeniedError>;

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError>;

    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDeniedError>;

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDeniedError>;

    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError>;

    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDeniedError>;

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError>;

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDeniedError>;

    fn check_env(&self, var: &str) -> Result<(), PermissionDeniedError>;

    fn check_exec(&self) -> Result<(), PermissionDeniedError>;
}

macro_rules! impl_sys_permission_kinds {
    ($($kind:ident($name:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum SystemsPermissionKind {
            $(
                #[doc = stringify!($kind)]
                $kind,
            )+

            Other(String),
        }
        impl SystemsPermissionKind {
            #[must_use]
            pub fn new(s: &str) -> Self {
                match s {
                    $( $name => Self::$kind, )+
                    _ => Self::Other(s.to_string()),
                }
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$kind => $name, )+
                    Self::Other(s) => &s,
                }
            }
        }
    };
}

impl_sys_permission_kinds!(
    LoadAvg("loadavg"),
    Hostname("hostname"),
    OsRelease("osRelease"),
    Networkinterfaces("networkInterfaces"),
    StatFs("statfs"),
    GetPriority("getPriority"),
    SystemMemoryInfo("systemMemoryInfo"),
    Gid("gid"),
    Uid("uid"),
    OsUptime("osUptime"),
    SetPriority("setPriority"),
    UserInfo("userInfo"),
    GetEGid("getegid"),
    Cpus("cpus"),
    HomeDir("homeDir"),
    Inspector("inspector"),
);

#[derive(Clone, Debug)]
pub struct PermissionsContainer(pub Arc<dyn WebPermissions>);
impl deno_web::TimersPermission for PermissionsContainer {
    fn allow_hrtime(&mut self) -> bool {
        self.0.allow_hrtime()
    }
}
impl deno_fetch::FetchPermissions for PermissionsContainer {
    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        access_kind: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        match access_kind {
            deno_permissions::OpenAccessKind::Read
            | deno_permissions::OpenAccessKind::ReadNoFollow => {
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::Write
            | deno_permissions::OpenAccessKind::WriteNoFollow => {
                let p = self.0.check_write(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::ReadWrite
            | deno_permissions::OpenAccessKind::ReadWriteNoFollow => {
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
        }
    }

    fn check_net_vsock(
        &mut self,
        cid: u32,
        port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(&format!("vsock:{cid}"), None, api_name)?;
        Ok(())
    }
}
impl deno_net::NetPermissions for PermissionsContainer {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(host.0.as_ref(), host.1, api_name)?;
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        access_kind: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        match access_kind {
            deno_permissions::OpenAccessKind::Read
            | deno_permissions::OpenAccessKind::ReadNoFollow => {
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::Write
            | deno_permissions::OpenAccessKind::WriteNoFollow => {
                let p = self.0.check_write(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::ReadWrite
            | deno_permissions::OpenAccessKind::ReadWriteNoFollow => {
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
        }
    }

    fn check_vsock(
        &mut self,
        cid: u32,
        _port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(&format!("vsock:{cid}"), None, api_name)?;
        Ok(())
    }
}
