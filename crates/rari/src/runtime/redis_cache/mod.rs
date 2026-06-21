use deno_core::{Extension, OpState, extension, op2};
use deno_error::JsErrorBox;
use redis::AsyncCommands;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::runtime::ext::ExtensionTrait;

const DEFAULT_TTL_SECS: u64 = 60;
const MS_PER_SEC: u64 = 1_000;
const REDIS_TIMEOUT: Duration = Duration::from_secs(2);

extension!(
    rari_redis_cache,
    ops = [op_cache_remote_get, op_cache_remote_set],
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

        let url = remote.and_then(|remote| remote.url.clone());
        let default_ttl_secs = remote
            .map(|remote| remote.default_ttl_secs)
            .unwrap_or(DEFAULT_TTL_SECS);

        Self {
            url,
            default_ttl_secs,
            connection: Mutex::new(None),
        }
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
                        redis::ErrorKind::IoError,
                        "redis connect timeout",
                    )))
                })??;
        *connection = Some(new_connection);

        Ok(connection
            .as_ref()
            .expect("redis connection was just initialized")
            .clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RedisCacheError {
    #[error("redis cache state is not initialized")]
    StateMissing,
    #[error("redis cache is not configured")]
    NotConfigured,
    #[error("redis connect: {0}")]
    Connect(#[from] redis::RedisError),
}

fn ttl_ms_to_secs(ttl_ms: u32) -> u64 {
    (ttl_ms as u64)
        .saturating_add(MS_PER_SEC - 1)
        .saturating_div(MS_PER_SEC)
        .max(1)
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
pub async fn op_cache_remote_get(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
) -> Result<Option<String>, JsErrorBox> {
    let mut connection = get_redis_state(state)
        .await
        .map_err(js_error)?
        .connection()
        .await
        .map_err(js_error)?;
    let raw: Option<Vec<u8>> = tokio::time::timeout(REDIS_TIMEOUT, connection.get(&key))
        .await
        .map_err(|_| js_error("redis get timeout"))?
        .map_err(js_error)?;
    Ok(raw.and_then(|bytes| String::from_utf8(bytes).ok()))
}

#[op2]
pub async fn op_cache_remote_set(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
    #[string] value: String,
    #[smi] ttl_ms: u32,
) -> Result<(), JsErrorBox> {
    let redis_state = get_redis_state(state).await.map_err(js_error)?;
    let mut connection = redis_state.connection().await.map_err(js_error)?;
    let ttl_secs = if ttl_ms == 0 {
        redis_state.default_ttl_secs
    } else {
        ttl_ms_to_secs(ttl_ms)
    };
    tokio::time::timeout(
        REDIS_TIMEOUT,
        connection.set_ex::<_, _, ()>(&key, value.into_bytes(), ttl_secs),
    )
    .await
    .map_err(|_| js_error("redis set timeout"))?
    .map_err(js_error)
}
