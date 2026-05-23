use super::ExtensionTrait;
use deno_core::{Extension, extension};
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{NpmPackageFolderResolverImpl, Resolver};
use std::sync::Arc;
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

        deno_node::deno_node::init::<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>(
            Some(services),
            fs,
        )
    }
}

pub fn extensions(resolver: Arc<Resolver>, is_snapshot: bool) -> Vec<Extension> {
    let mut node_ext = deno_node::deno_node::build(resolver.clone(), false);
    node_ext.lazy_loaded_esm_files = std::borrow::Cow::Borrowed(&[]);
    node_ext.lazy_loaded_js_files = std::borrow::Cow::Borrowed(&[]);

    let init_ext = init_node::build((), is_snapshot);

    vec![node_ext, init_ext]
}
