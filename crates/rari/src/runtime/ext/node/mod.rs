use std::sync::Arc;

use deno_core::{Extension, ExtensionArguments, extension};
use deno_node::deno_node;
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{NpmPackageFolderResolverImpl, Resolver};
use sys_traits::impls::RealSys;

use super::{ExtensionTrait, lazy};

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
        Self::init()
    }
}

impl ExtensionTrait<Arc<Resolver>> for deno_node {
    const LAZY_INIT: bool = true;

    fn init(resolver: Arc<Resolver>) -> Extension {
        let services = resolver.init_services();
        let fs = resolver.filesystem();
        Self::init::<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>(
            Some(services),
            fs,
        )
    }

    fn lazy_init() -> Extension {
        Self::lazy_init::<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>()
    }

    fn lazy_args(resolver: Arc<Resolver>) -> ExtensionArguments {
        let services = resolver.init_services();
        let fs = resolver.filesystem();
        Self::args::<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>(
            Some(services),
            fs,
        )
    }
}

pub fn extensions(
    resolver: Arc<Resolver>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<Arc<Resolver>, deno_node>(
        resolver,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), init_node>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
