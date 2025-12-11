use super::ExtensionTrait;
use deno_core::{Extension, extension};
use deno_kv::{dynamic::MultiBackendDbHandler, remote::RemoteDbHandler, sqlite::SqliteDbHandler};
use std::path::PathBuf;

extension!(
    init_kv,
    deps = [rari],
    esm_entry_point = "ext:init_kv/init_kv.js",
    esm = [ dir "src/runtime/ext/kv", "init_kv.js" ],
);
impl ExtensionTrait<()> for init_kv {
    fn init((): ()) -> Extension {
        init_kv::init()
    }
}
impl ExtensionTrait<KvStore> for deno_kv::deno_kv {
    fn init(store: KvStore) -> Extension {
        deno_kv::deno_kv::init(store.handler(), store.config())
    }
}

pub fn extensions(store: KvStore, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_kv::deno_kv::build(store, is_snapshot), init_kv::build((), is_snapshot)]
}

#[derive(Clone)]
enum KvStoreBuilder {
    Local { path: Option<PathBuf>, rng_seed: Option<u64> },
    Remote { http_options: deno_kv::remote::HttpOptions },
}

#[derive(Clone, Copy)]
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
impl From<KvConfig> for deno_kv::KvConfig {
    fn from(value: KvConfig) -> Self {
        deno_kv::KvConfigBuilder::default()
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

        KvConfig {
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

    pub fn new_remote(http_options: deno_kv::remote::HttpOptions, config: KvConfig) -> Self {
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

    pub fn config(&self) -> deno_kv::KvConfig {
        self.1.into()
    }
}
impl Default for KvStore {
    fn default() -> Self {
        Self::new_local(None, None, KvConfig::default())
    }
}
