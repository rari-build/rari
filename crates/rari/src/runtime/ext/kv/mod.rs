use std::path::PathBuf;

use ::deno_kv::{
    KvConfig as DenoKvConfig, KvConfigBuilder,
    dynamic::MultiBackendDbHandler,
    remote::{HttpOptions, RemoteDbHandler},
    sqlite::SqliteDbHandler,
};
use deno_core::{Extension, ExtensionArguments, extension};
use deno_kv::deno_kv;

use super::{ExtensionTrait, lazy};

extension!(
    init_kv,
    deps = [rari],
    esm_entry_point = "ext:init_kv/init_kv.ts",
    esm = [ dir "src/runtime/ext/kv", "init_kv.ts" ],
);

impl ExtensionTrait<()> for init_kv {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<KvStore> for deno_kv {
    const LAZY_INIT: bool = true;

    fn init(store: KvStore) -> Extension {
        Self::init(Box::new(store.handler()), store.config())
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(store: KvStore) -> ExtensionArguments {
        Self::args(Box::new(store.handler()), store.config())
    }
}

pub fn extensions(store: KvStore, is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<KvStore, deno_kv>(store, is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_kv>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}

#[derive(Clone)]
enum KvStoreBuilder {
    Local { path: Option<PathBuf>, rng_seed: Option<u64> },
    Remote { http_options: HttpOptions },
}

#[derive(Clone, Copy)]
#[expect(clippy::struct_field_names, reason = "All fields are max limits by design")]
pub struct KvConfig {
    pub max_write_key_size_bytes: usize,
    pub max_value_size_bytes: usize,
    pub max_read_ranges: usize,
    pub max_read_entries: usize,
    pub max_checks: usize,
    pub max_mutations: usize,
    pub max_watched_keys: usize,
    pub max_total_mutation_size_bytes: usize,
    pub max_total_key_size_bytes: usize,
}

impl From<KvConfig> for DenoKvConfig {
    fn from(value: KvConfig) -> Self {
        KvConfigBuilder::default()
            .max_write_key_size_bytes(value.max_write_key_size_bytes)
            .max_value_size_bytes(value.max_value_size_bytes)
            .max_read_ranges(value.max_read_ranges)
            .max_read_entries(value.max_read_entries)
            .max_checks(value.max_checks)
            .max_mutations(value.max_mutations)
            .max_watched_keys(value.max_watched_keys)
            .max_total_mutation_size_bytes(value.max_total_mutation_size_bytes)
            .max_total_key_size_bytes(value.max_total_key_size_bytes)
            .build()
    }
}

impl Default for KvConfig {
    fn default() -> Self {
        const MAX_WRITE_KEY_SIZE_BYTES: usize = 2048;
        const MAX_VALUE_SIZE_BYTES: usize = 65536;
        const MAX_READ_RANGES: usize = 10;
        const MAX_READ_ENTRIES: usize = 1000;
        const MAX_CHECKS: usize = 100;
        const MAX_MUTATIONS: usize = 1000;
        const MAX_WATCHED_KEYS: usize = 10;
        const MAX_TOTAL_MUTATION_SIZE_BYTES: usize = 800 * 1024;
        const MAX_TOTAL_KEY_SIZE_BYTES: usize = 80 * 1024;

        Self {
            max_write_key_size_bytes: MAX_WRITE_KEY_SIZE_BYTES,
            max_value_size_bytes: MAX_VALUE_SIZE_BYTES,
            max_read_ranges: MAX_READ_RANGES,
            max_read_entries: MAX_READ_ENTRIES,
            max_checks: MAX_CHECKS,
            max_mutations: MAX_MUTATIONS,
            max_watched_keys: MAX_WATCHED_KEYS,
            max_total_mutation_size_bytes: MAX_TOTAL_MUTATION_SIZE_BYTES,
            max_total_key_size_bytes: MAX_TOTAL_KEY_SIZE_BYTES,
        }
    }
}

#[derive(Clone)]
pub struct KvStore(KvStoreBuilder, KvConfig);
impl KvStore {
    pub fn new_local(path: Option<PathBuf>, rng_seed: Option<u64>, config: KvConfig) -> Self {
        Self(KvStoreBuilder::Local { path, rng_seed }, config)
    }

    pub fn new_remote(http_options: HttpOptions, config: KvConfig) -> Self {
        Self(KvStoreBuilder::Remote { http_options }, config)
    }

    pub fn handler(&self) -> MultiBackendDbHandler {
        match &self.0 {
            KvStoreBuilder::Local { path, rng_seed } => {
                let db = SqliteDbHandler::new(path.clone(), *rng_seed);
                MultiBackendDbHandler::new(vec![(&[""], Box::new(db))])
            }

            KvStoreBuilder::Remote { http_options } => {
                let db = RemoteDbHandler::new(http_options.clone());
                MultiBackendDbHandler::new(vec![(&["https://", "http://"], Box::new(db))])
            }
        }
    }

    pub fn config(&self) -> DenoKvConfig {
        self.1.into()
    }
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new_local(None, None, KvConfig::default())
    }
}
