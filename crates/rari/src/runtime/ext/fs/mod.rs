use std::{borrow::Cow, path::Path};

use super::{ExtensionTrait, web::PermissionsContainer};
use deno_core::{Extension, extension};
use deno_fs::FileSystemRc;

use deno_permissions::{CheckedPath, OpenAccessKind, PermissionCheckError};

extension!(
    init_fs,
    deps = [rari],
    esm_entry_point = "ext:init_fs/init_fs.js",
    esm = [ dir "src/runtime/ext/fs", "init_fs.js" ],
);

impl ExtensionTrait<()> for init_fs {
    fn init((): ()) -> Extension {
        init_fs::init()
    }
}

impl ExtensionTrait<FileSystemRc> for deno_fs::deno_fs {
    fn init(fs: FileSystemRc) -> Extension {
        deno_fs::deno_fs::init::<PermissionsContainer>(fs)
    }
}

pub fn extensions(fs: FileSystemRc, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_fs::deno_fs::build(fs, is_snapshot), init_fs::build((), is_snapshot)]
}

impl deno_fs::FsPermissions for PermissionsContainer {
    fn check_open<'a>(
        &self,
        path: Cow<'a, Path>,
        access_kind: OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        match access_kind {
            OpenAccessKind::Read => {
                self.0.check_read_all(Some(api_name))?;
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
            OpenAccessKind::Write => {
                self.0.check_write_all(api_name)?;
                let p = self.0.check_write(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
            OpenAccessKind::ReadWrite => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_write_all(api_name)?;
                // For read-write, check read permission and use that path
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
            OpenAccessKind::ReadNoFollow => {
                self.0.check_read_all(Some(api_name))?;
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
            OpenAccessKind::WriteNoFollow => {
                self.0.check_write_all(api_name)?;
                let p = self.0.check_write(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
            OpenAccessKind::ReadWriteNoFollow => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_write_all(api_name)?;
                // For read-write, check read permission and use that path
                let p = self.0.check_read(path, Some(api_name))?;
                Ok(CheckedPath::unsafe_new(p))
            }
        }
    }

    fn check_open_blind<'a>(
        &self,
        path: Cow<'a, Path>,
        access_kind: OpenAccessKind,
        display: &str,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        match access_kind {
            OpenAccessKind::Read => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_read_blind(&path, display, api_name)?;
            }
            OpenAccessKind::Write => {
                self.0.check_write_all(api_name)?;
                self.0.check_write_blind(&path, display, api_name)?;
            }
            OpenAccessKind::ReadWrite => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_write_all(api_name)?;
                self.0.check_read_blind(&path, display, api_name)?;
                self.0.check_write_blind(&path, display, api_name)?;
            }
            OpenAccessKind::ReadNoFollow => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_read_blind(&path, display, api_name)?;
            }
            OpenAccessKind::WriteNoFollow => {
                self.0.check_write_all(api_name)?;
                self.0.check_write_blind(&path, display, api_name)?;
            }
            OpenAccessKind::ReadWriteNoFollow => {
                self.0.check_read_all(Some(api_name))?;
                self.0.check_write_all(api_name)?;
                self.0.check_read_blind(&path, display, api_name)?;
                self.0.check_write_blind(&path, display, api_name)?;
            }
        }
        Ok(CheckedPath::unsafe_new(path))
    }

    fn check_read_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        self.0.check_read_all(Some(api_name))?;
        Ok(())
    }

    fn check_write_partial<'a>(
        &self,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        let p = self.0.check_write_partial(&path.to_string_lossy(), api_name)?;
        Ok(CheckedPath::unsafe_new(Cow::Owned(p)))
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        self.0.check_write_all(api_name)?;
        Ok(())
    }
}
