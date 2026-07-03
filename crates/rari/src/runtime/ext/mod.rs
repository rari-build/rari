use std::{borrow::Cow::Borrowed, option::Option::None, path::PathBuf, sync::Arc};

use deno_bundle_runtime::deno_bundle_runtime;
use deno_core::Extension;
use deno_fs::{FileSystemRc, RealFs, sync::MaybeArc};
use deno_io::Stdio;
use deno_runtime::deno_canvas::deno_canvas;
use deno_web::InMemoryBroadcastChannel;

use crate::runtime::ext::{kv::KvStore, node::resolvers::Resolver, web::WebOptions};

pub trait ExtensionTrait<A> {
    fn init(options: A) -> Extension;

    fn for_warmup(mut ext: Extension) -> Extension {
        ext.js_files = Borrowed(&[]);
        ext.esm_files = Borrowed(&[]);
        ext.esm_entry_point = None;

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
mod napi;
mod node;
mod node_crypto;
mod node_sqlite;
mod rari;
mod runtime;
mod utilities;
mod web;
mod webgpu;
mod webidl;
mod websocket;
mod webstorage;

#[derive(Clone)]
#[non_exhaustive]
pub struct ExtensionOptions {
    pub web: WebOptions,
    pub io_pipes: Option<Stdio>,
    pub cache: Option<()>,
    pub filesystem: FileSystemRc,
    pub crypto_seed: Option<u64>,
    pub node_resolver: Arc<Resolver>,
    pub broadcast_channel: InMemoryBroadcastChannel,
    pub webstorage_origin_storage_dir: Option<PathBuf>,
    pub kv_store: KvStore,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            web: WebOptions::default(),
            io_pipes: Some(Stdio::default()),
            filesystem: MaybeArc::new(RealFs),
            cache: Some(()),
            crypto_seed: None,
            node_resolver: Arc::new(Resolver::default()),
            broadcast_channel: InMemoryBroadcastChannel::default(),
            webstorage_origin_storage_dir: None,
            kv_store: KvStore::default(),
        }
    }
}

pub fn extensions(options: &ExtensionOptions, is_snapshot: bool) -> Vec<Extension> {
    let mut extensions = Vec::new();

    extensions.extend(utilities::extensions(is_snapshot));
    extensions.extend(webidl::extensions(is_snapshot));
    extensions.extend(web::extensions(options.web.clone(), is_snapshot));
    extensions.extend(rari::extensions(is_snapshot));
    extensions.extend(rari::redis_cache_extensions(is_snapshot));
    extensions.extend(cache::extensions(options.cache, is_snapshot));
    extensions.extend(crypto::extensions(options.crypto_seed, is_snapshot));
    extensions.extend(fs::extensions(Arc::clone(&options.filesystem), is_snapshot));
    extensions.extend(io::extensions(options.io_pipes.clone(), is_snapshot));
    extensions
        .extend(webstorage::extensions(options.webstorage_origin_storage_dir.clone(), is_snapshot));
    extensions.extend(websocket::extensions(options.web.clone(), is_snapshot));
    extensions.extend(http::extensions((), is_snapshot));
    extensions.extend(fetch::extensions(is_snapshot, None));
    extensions.extend(ffi::extensions(is_snapshot));
    extensions.extend(kv::extensions(options.kv_store.clone(), is_snapshot));
    extensions.extend(webgpu::extensions(is_snapshot));
    {
        let mut canvas_ext = deno_canvas::init();
        if is_snapshot {
            canvas_ext.js_files = Borrowed(&[]);
            canvas_ext.esm_files = Borrowed(&[]);
            canvas_ext.esm_entry_point = None;
        }
        extensions.push(canvas_ext);
    }
    extensions.extend(cron::extensions(is_snapshot));
    extensions.extend(napi::extensions(is_snapshot));
    extensions.extend(node_crypto::extensions(is_snapshot));
    extensions.extend(node_sqlite::extensions(is_snapshot));
    extensions.extend(node::extensions(Arc::clone(&options.node_resolver), is_snapshot));
    {
        let mut bundle_ext = deno_bundle_runtime::init(None);
        bundle_ext.esm_files = Borrowed(&[]);
        bundle_ext.esm_entry_point = None;
        extensions.push(bundle_ext);
    }
    extensions.extend(runtime::extensions(options, None, is_snapshot));

    extensions
}
