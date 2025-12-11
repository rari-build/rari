use deno_core::Extension;
use deno_fs::{FileSystemRc, sync::MaybeArc};

trait ExtensionTrait<A> {
    fn init(options: A) -> Extension;

    fn for_warmup(mut ext: Extension) -> Extension {
        ext.js_files = ::std::borrow::Cow::Borrowed(&[]);
        ext.esm_files = ::std::borrow::Cow::Borrowed(&[]);
        ext.esm_entry_point = ::std::option::Option::None;

        ext
    }

    fn build(options: A, is_snapshot: bool) -> Extension {
        let ext: Extension = Self::init(options);
        if is_snapshot { Self::for_warmup(ext) } else { ext }
    }
}

mod cache;
mod cron;
mod crypto;
mod fetch;
mod ffi;
mod fs;
mod http;
mod io;
mod kv;
mod module_reload;
mod napi;
mod node;
mod promise_manager;
mod rari;
mod react;
mod rsc_modules;
mod rsc_renderer;
mod runtime;
mod server_functions;
mod streaming;
mod web;
mod webgpu;
mod webidl;
mod websocket;
mod webstorage;

#[derive(Clone)]
pub struct ExtensionOptions {
    pub web: web::WebOptions,
    pub io_pipes: Option<deno_io::Stdio>,
    pub cache: Option<()>,
    pub filesystem: FileSystemRc,
    pub crypto_seed: Option<u64>,
    pub node_resolver: std::sync::Arc<node::resolvers::Resolver>,
    pub broadcast_channel: deno_web::InMemoryBroadcastChannel,
    pub webstorage_origin_storage_dir: Option<std::path::PathBuf>,
    pub kv_store: kv::KvStore,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            web: web::WebOptions::default(),
            io_pipes: Some(deno_io::Stdio::default()),
            filesystem: MaybeArc::new(deno_fs::RealFs),
            cache: Some(()),
            crypto_seed: None,
            node_resolver: std::sync::Arc::new(node::resolvers::Resolver::default()),
            broadcast_channel: deno_web::InMemoryBroadcastChannel::default(),
            webstorage_origin_storage_dir: None,
            kv_store: kv::KvStore::default(),
        }
    }
}

pub(crate) fn extensions(options: &ExtensionOptions, is_snapshot: bool) -> Vec<Extension> {
    let mut extensions = Vec::new();

    extensions.extend(rari::extensions(is_snapshot));
    extensions.extend(promise_manager::extensions(is_snapshot));
    extensions.extend(module_reload::extensions(is_snapshot));
    extensions.extend(rsc_modules::extensions(is_snapshot));
    extensions.extend(server_functions::extensions(is_snapshot));
    extensions.extend(react::extensions(is_snapshot));
    extensions.extend(rsc_renderer::extensions(is_snapshot));
    extensions.extend(webidl::extensions(is_snapshot));
    extensions.extend(web::extensions(options.web.clone(), is_snapshot));
    extensions.extend(cache::extensions(options.cache, is_snapshot));
    extensions.extend(crypto::extensions(options.crypto_seed, is_snapshot));
    extensions.extend(fs::extensions(options.filesystem.clone(), is_snapshot));
    extensions.extend(io::extensions(options.io_pipes.clone(), is_snapshot));
    extensions
        .extend(webstorage::extensions(options.webstorage_origin_storage_dir.clone(), is_snapshot));
    extensions.extend(websocket::extensions(options.web.clone(), is_snapshot));
    extensions.extend(http::extensions((), is_snapshot));
    extensions.extend(streaming::extensions(is_snapshot));
    extensions.extend(fetch::extensions(is_snapshot));
    extensions.extend(ffi::extensions(is_snapshot));
    extensions.extend(kv::extensions(options.kv_store.clone(), is_snapshot));
    extensions.extend(webgpu::extensions(is_snapshot));
    extensions.extend(cron::extensions(is_snapshot));
    extensions.extend(napi::extensions(is_snapshot));
    extensions.extend(node::extensions(options.node_resolver.clone(), is_snapshot));
    extensions.push(deno_bundle_runtime::deno_bundle_runtime::init(None));
    extensions.extend(runtime::extensions(options, None, is_snapshot));

    extensions
}
