use std::sync::Arc;

use deno_core::{Extension, extension};
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{NpmPackageFolderResolverImpl, Resolver};
use sys_traits::impls::RealSys;

use super::ExtensionTrait;

mod cjs_translator;
pub mod resolvers;

extension!(
    init_node,
    deps = [rari],
    esm_entry_point = "ext:init_node/init_node.ts",
    esm = [ dir "src/runtime/ext/node", "init_node.ts" ],
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

        deno_node::deno_node::init::<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>(
            Some(services),
            fs,
        )
    }
}

pub fn extensions(resolver: Arc<Resolver>, is_snapshot: bool) -> Vec<Extension> {
    let node_ext = deno_node::deno_node::build(resolver, is_snapshot);

    let init_ext = init_node::build((), is_snapshot);

    vec![node_ext, init_ext]
}
