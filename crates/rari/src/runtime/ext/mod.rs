use std::{borrow::Cow::Borrowed, option::Option::None, sync::Arc};

use ::deno_bundle_runtime::{BundleProvider, deno_bundle_runtime};
use deno_core::{Extension, ExtensionArguments};
use deno_fs::{FileSystemRc, RealFs, sync::MaybeArc};
use deno_io::Stdio;
pub use node::{
    cjs_translator::NodeCodeTranslator,
    resolvers::{NpmPackageFolderResolverImpl, Resolver},
};

use crate::runtime::ext::web::WebOptions;

pub trait ExtensionTrait<A> {
    const LAZY_INIT: bool = false;

    fn init(options: A) -> Extension;

    #[expect(
        clippy::panic,
        reason = "Only called when LAZY_INIT is true; lazy extensions override this"
    )]
    fn lazy_init() -> Extension {
        panic!("lazy_init is not implemented for this extension")
    }

    #[expect(
        clippy::panic,
        reason = "Only called when LAZY_INIT is true; lazy extensions override this"
    )]
    fn lazy_args(_options: A) -> ExtensionArguments {
        panic!("lazy_args is not implemented for this extension")
    }

    fn for_warmup(mut ext: Extension) -> Extension {
        ext.js_files = Borrowed(&[]);
        ext.esm_files = Borrowed(&[]);
        ext.esm_entry_point = None;

        ext
    }

    fn build(options: A, is_snapshot: bool) -> Extension {
        let ext = if Self::LAZY_INIT { Self::lazy_init() } else { Self::init(options) };
        if is_snapshot { Self::for_warmup(ext) } else { ext }
    }

    fn register(
        options: A,
        is_snapshot: bool,
        extensions: &mut Vec<Extension>,
        lazy_args: &mut Vec<ExtensionArguments>,
    ) where
        A: Clone,
    {
        if Self::LAZY_INIT {
            lazy_args.push(Self::lazy_args(options.clone()));
        }
        extensions.push(Self::build(options, is_snapshot));
    }
}

mod crypto;
mod fetch;
mod fs;
mod http;
mod io;
mod lazy;
mod napi;
mod node;
mod node_crypto;
mod node_sqlite;
mod rari;
mod runtime;
mod utilities;
mod web;
mod webidl;
mod websocket;

impl ExtensionTrait<Option<Arc<dyn BundleProvider>>> for deno_bundle_runtime {
    const LAZY_INIT: bool = true;

    fn init(provider: Option<Arc<dyn BundleProvider>>) -> Extension {
        Self::init(provider)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(provider: Option<Arc<dyn BundleProvider>>) -> ExtensionArguments {
        Self::args(provider)
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct ExtensionOptions {
    pub web: WebOptions,
    pub io_pipes: Option<Stdio>,
    pub filesystem: FileSystemRc,
    pub crypto_seed: Option<u64>,
    pub node_resolver: Arc<Resolver>,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            web: WebOptions::default(),
            io_pipes: Some(Stdio::default()),
            filesystem: MaybeArc::new(RealFs),
            crypto_seed: None,
            node_resolver: Arc::new(Resolver::default()),
        }
    }
}

pub fn extensions(options: &ExtensionOptions, is_snapshot: bool) -> Vec<Extension> {
    extensions_with_lazy_args(options, is_snapshot).0
}

pub fn extensions_with_lazy_args(
    options: &ExtensionOptions,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();

    lazy::merge(&mut extensions, &mut lazy_args, utilities::extensions(is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, webidl::extensions(is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, web::extensions(options.web.clone(), is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, rari::extensions(is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, rari::cache::extensions(is_snapshot));
    lazy::merge(
        &mut extensions,
        &mut lazy_args,
        crypto::extensions(options.crypto_seed, is_snapshot),
    );
    lazy::merge(
        &mut extensions,
        &mut lazy_args,
        fs::extensions(Arc::clone(&options.filesystem), is_snapshot),
    );
    lazy::merge(
        &mut extensions,
        &mut lazy_args,
        io::extensions(options.io_pipes.clone(), is_snapshot),
    );
    lazy::merge(
        &mut extensions,
        &mut lazy_args,
        websocket::extensions(options.web.clone(), is_snapshot),
    );
    lazy::merge(&mut extensions, &mut lazy_args, http::extensions((), is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, fetch::extensions(is_snapshot, None));
    lazy::merge(&mut extensions, &mut lazy_args, napi::extensions(is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, node_crypto::extensions(is_snapshot));
    lazy::merge(&mut extensions, &mut lazy_args, node_sqlite::extensions(is_snapshot));
    lazy::merge(
        &mut extensions,
        &mut lazy_args,
        node::extensions(Arc::clone(&options.node_resolver), is_snapshot),
    );
    lazy::register::<Option<Arc<dyn BundleProvider>>, deno_bundle_runtime>(
        None,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    if let Some(bundle_ext) = extensions.last_mut() {
        bundle_ext.esm_files = Borrowed(&[]);
        bundle_ext.esm_entry_point = None;
    }
    lazy::merge(&mut extensions, &mut lazy_args, runtime::extensions(options, is_snapshot));

    (extensions, lazy_args)
}

pub fn lazy_extension_args(options: &ExtensionOptions) -> Vec<ExtensionArguments> {
    extensions_with_lazy_args(options, false).1
}
