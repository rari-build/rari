use super::{
    ExtensionTrait,
    web::{PermissionsContainer, SystemsPermissionKind},
};
use deno_core::{Extension, extension};
use deno_node::NodePermissions;
use deno_permissions::PermissionCheckError;
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{NpmPackageFolderResolverImpl, Resolver};
use std::{borrow::Cow, path::Path, sync::Arc};
use sys_traits::impls::RealSys;

mod cjs_translator;
pub mod resolvers;

extension!(
    init_node,
    deps = [rari],
    esm_entry_point = "ext:init_node/init_node.js",
    esm = [ dir "src/runtime/ext/node", "init_node.js" ],
);

impl ExtensionTrait<()> for init_node {
    fn init((): ()) -> Extension {
        init_node::init()
    }
}

impl ExtensionTrait<Arc<Resolver>> for deno_node::deno_node {
    fn init(resolver: Arc<Resolver>) -> Extension {
        let services = resolver.init_services();

        let fs = resolver.filesystem();

        deno_node::deno_node::init::<
            PermissionsContainer,
            DenoInNpmPackageChecker,
            NpmPackageFolderResolverImpl,
            RealSys,
        >(Some(services), fs)
    }
}

pub fn extensions(resolver: Arc<Resolver>, is_snapshot: bool) -> Vec<Extension> {
    let node_ext = deno_node::deno_node::build(resolver.clone(), is_snapshot);

    let init_ext = init_node::build((), is_snapshot);

    vec![node_ext, init_ext]
}

impl NodePermissions for PermissionsContainer {
    fn check_net(
        &mut self,
        host: (&str, Option<u16>),
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(host.0, host.1, api_name)?;
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        access_kind: deno_permissions::OpenAccessKind,
        api_name: Option<&str>,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        match access_kind {
            deno_permissions::OpenAccessKind::Read
            | deno_permissions::OpenAccessKind::ReadNoFollow => {
                let p = self.0.check_read(path, api_name)?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::Write
            | deno_permissions::OpenAccessKind::WriteNoFollow => {
                let p = self.0.check_write(path, api_name)?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
            deno_permissions::OpenAccessKind::ReadWrite
            | deno_permissions::OpenAccessKind::ReadWriteNoFollow => {
                let p = self.0.check_read(path, api_name)?;
                Ok(deno_permissions::CheckedPath::unsafe_new(p))
            }
        }
    }

    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> std::result::Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }

    fn query_read_all(&mut self) -> bool {
        self.0.check_read_all(None).is_ok()
    }

    fn check_sys(&mut self, kind: &str, api_name: &str) -> Result<(), PermissionCheckError> {
        let kind = SystemsPermissionKind::new(kind);
        self.0.check_sys(kind, api_name)?;
        Ok(())
    }
}
