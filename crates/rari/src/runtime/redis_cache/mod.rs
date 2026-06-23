#![allow(clippy::exhaustive_structs)]

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use deno_core::{Extension, OpState, extension, op2};
use deno_error::JsErrorBox;
use redis::AsyncCommands;
use tokio::sync::Mutex;

use crate::runtime::ext::ExtensionTrait;

const DEFAULT_TTL_SECS: u64 = 60;
const MS_PER_SEC: u64 = 1_000;
const REDIS_TIMEOUT: Duration = Duration::from_secs(2);

pub const TEST_REDIS_URL: &str = "redis://localhost:6379";

extension!(
    rari_redis_cache,
    ops = [op_redis_cache_get, op_redis_cache_set],
    options = {},
    state = |state, _options| {
        state.put(Arc::new(RedisCacheState::from_config()));
    },
);

impl ExtensionTrait<()> for rari_redis_cache {
    fn init((): ()) -> Extension {
        rari_redis_cache::init()
    }
}

pub fn extensions(_options: Option<()>, is_snapshot: bool) -> Vec<Extension> {
    vec![rari_redis_cache::build((), is_snapshot)]
}

pub struct RedisCacheState {
    url: Option<String>,
    default_ttl_secs: u64,
    connection: Mutex<Option<redis::aio::MultiplexedConnection>>,
}

impl RedisCacheState {
    pub fn from_config() -> Self {
        let remote = crate::server::config::Config::get()
            .and_then(|config| config.use_cache.remote.as_ref());

        let url = remote.and_then(|r| match r.handler.as_str() {
            "test" => Some(TEST_REDIS_URL.to_string()),
            _ => r.url.clone(),
        });
        let default_ttl_secs =
            remote.map(|remote| remote.default_ttl_secs).unwrap_or(DEFAULT_TTL_SECS);

        Self { url, default_ttl_secs, connection: Mutex::new(None) }
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, RedisCacheError> {
        let Some(url) = self.url.as_deref() else {
            return Err(RedisCacheError::NotConfigured);
        };

        let mut connection = self.connection.lock().await;
        if let Some(connection) = connection.as_ref() {
            return Ok(connection.clone());
        }

        let client = redis::Client::open(url)?;
        let new_connection =
            tokio::time::timeout(REDIS_TIMEOUT, client.get_multiplexed_async_connection())
                .await
                .map_err(|_| {
                RedisCacheError::Connect(redis::RedisError::from((
                    redis::ErrorKind::Io,
                    "redis connect timeout",
                )))
            })??;
        *connection = Some(new_connection.clone());

        Ok(new_connection)
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedisCacheError {
    #[error("redis cache state is not initialized")]
    StateMissing,
    #[error("redis cache is not configured")]
    NotConfigured,
    #[error("redis connect: {0}")]
    Connect(#[from] redis::RedisError),
}

fn ttl_ms_to_secs(ttl_ms: u32) -> u64 {
    u64::from(ttl_ms).saturating_add(MS_PER_SEC - 1).saturating_div(MS_PER_SEC).max(1)
}

fn js_error(error: impl std::fmt::Display) -> JsErrorBox {
    JsErrorBox::generic(error.to_string())
}

async fn get_redis_state(
    state: Rc<RefCell<OpState>>,
) -> Result<Arc<RedisCacheState>, RedisCacheError> {
    state
        .borrow()
        .try_borrow::<Arc<RedisCacheState>>()
        .cloned()
        .ok_or(RedisCacheError::StateMissing)
}

#[op2]
#[string]
pub async fn op_redis_cache_get(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
) -> Result<Option<String>, JsErrorBox> {
    let mut connection =
        get_redis_state(state).await.map_err(js_error)?.connection().await.map_err(js_error)?;
    let raw: Option<Vec<u8>> = tokio::time::timeout(REDIS_TIMEOUT, connection.get(&key))
        .await
        .map_err(|_| js_error("redis get timeout"))?
        .map_err(js_error)?;
    Ok(raw.and_then(|bytes| String::from_utf8(bytes).ok()))
}

#[op2]
pub async fn op_redis_cache_set(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
    #[string] value: String,
    #[smi] ttl_ms: u32,
) -> Result<(), JsErrorBox> {
    let redis_state = get_redis_state(state).await.map_err(js_error)?;
    let mut connection = redis_state.connection().await.map_err(js_error)?;
    let ttl_secs = if ttl_ms == 0 { redis_state.default_ttl_secs } else { ttl_ms_to_secs(ttl_ms) };
    let bytes = value.into_bytes();
    let fut: redis::RedisFuture<'_, ()> = if ttl_secs == 0 {
        connection.set::<_, _, ()>(&key, bytes)
    } else {
        connection.set_ex::<_, _, ()>(&key, bytes, ttl_secs)
    };
    tokio::time::timeout(REDIS_TIMEOUT, fut)
        .await
        .map_err(|_| js_error("redis set timeout"))?
        .map_err(js_error)
}
