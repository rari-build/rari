use std::{
    cell::RefCell,
    env,
    fmt::Display,
    fs::create_dir_all,
    io::Error,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use deno_core::{Extension, OpState, extension, op2};
use deno_error::JsErrorBox;
use redb::ReadableDatabase;
use tokio::task::{JoinError, spawn_blocking};

use crate::{runtime::ext::ExtensionTrait, server::config::Config};

const DEFAULT_TTL_SECS: u64 = 60;
const EXPIRY_NEVER: u64 = 0;
const TABLE_NAME: &str = "use_cache_remote_entries";

fn test_redb_path() -> PathBuf {
    env::temp_dir().join("rari-use-cache-redb-test.redb")
}

const TABLE_DEFINITION: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(TABLE_NAME);

extension!(
    rari_redb_cache,
    ops = [op_redb_cache_get, op_redb_cache_set],
    options = {},
    state = |state, _options| {
        state.put(Arc::new(RedbCacheState::from_config()));
    },
);

impl ExtensionTrait<()> for rari_redb_cache {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(_options: Option<()>, is_snapshot: bool) -> Vec<Extension> {
    vec![rari_redb_cache::build((), is_snapshot)]
}

pub struct RedbCacheState {
    path: Option<PathBuf>,
    default_ttl_secs: u64,
    database: parking_lot::Mutex<Option<Arc<redb::Database>>>,
}

impl RedbCacheState {
    pub fn from_config() -> Self {
        let remote = Config::get().and_then(|config| config.use_cache.remote.as_ref());

        let path = remote.and_then(|remote| match remote.handler.as_str() {
            "test" => Some(test_redb_path()),
            _ => remote.url.as_ref().map(PathBuf::from),
        });
        let default_ttl_secs =
            remote.map(|remote| remote.default_ttl_secs).unwrap_or(DEFAULT_TTL_SECS);

        Self { path, default_ttl_secs, database: parking_lot::Mutex::new(None) }
    }

    fn database(&self) -> Result<Arc<redb::Database>, RedbCacheError> {
        let Some(path) = self.path.as_ref() else {
            return Err(RedbCacheError::NotConfigured);
        };

        let mut guard = self.database.lock();
        if let Some(db) = guard.as_ref() {
            return Ok(Arc::clone(db));
        }

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            create_dir_all(parent)?;
        }

        let db = redb::Database::create(path).map_err(redb::Error::from)?;
        let arc = Arc::new(db);
        *guard = Some(Arc::clone(&arc));
        Ok(arc)
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedbCacheError {
    #[error("redb cache state is not initialized")]
    StateMissing,
    #[error("redb cache is not configured")]
    NotConfigured,
    #[error("redb: {0}")]
    Op(#[from] redb::Error),
    #[error("redb: {0}")]
    Table(#[from] redb::TableError),
    #[error("redb: {0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("redb: {0}")]
    Commit(#[from] redb::CommitError),
    #[error("redb: {0}")]
    Storage(#[from] redb::StorageError),
    #[error("redb: {0}")]
    Database(#[from] redb::DatabaseError),
    #[error("redb io: {0}")]
    Io(#[from] Error),
    #[error("redb join: {0}")]
    Join(#[from] JoinError),
}

fn expires_at_ms_from_ttl(ttl_ms: u32, default_ttl_secs: u64) -> Result<u64, RedbCacheError> {
    let ttl_duration_ms =
        if ttl_ms == 0 { default_ttl_secs.saturating_mul(1_000) } else { u64::from(ttl_ms) };

    if ttl_duration_ms == 0 {
        return Ok(EXPIRY_NEVER);
    }

    Ok(now_ms()?.saturating_add(ttl_duration_ms))
}

fn now_ms() -> Result<u64, RedbCacheError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RedbCacheError::Io(Error::other(e.to_string())))?
        .as_millis();
    u64::try_from(millis)
        .map_err(|_| RedbCacheError::Io(Error::other("system time millis exceeded u64")))
}

fn encode_entry(expires_at_ms: u64, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&expires_at_ms.to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

fn decode_entry(bytes: &[u8]) -> Option<(u64, &[u8])> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some((u64::from_be_bytes(arr), &bytes[8..]))
}

fn js_error(error: &impl Display) -> JsErrorBox {
    JsErrorBox::generic(error.to_string())
}

fn read_from_database(
    database: &Arc<redb::Database>,
    key: &str,
) -> Result<Option<String>, RedbCacheError> {
    let tx = database.begin_read()?;
    let table = match tx.open_table(TABLE_DEFINITION) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(err) => return Err(redb::Error::from(err).into()),
    };

    let Some(raw) = table.get(key)? else {
        return Ok(None);
    };
    let bytes = raw.value();

    let Some((expires_at_ms, payload)) = decode_entry(bytes) else {
        return Ok(None);
    };

    if expires_at_ms != EXPIRY_NEVER && now_ms()? >= expires_at_ms {
        drop(table);
        drop(tx);
        let _ = remove_expired(database, key);
        return Ok(None);
    }

    Ok(String::from_utf8(payload.to_vec()).ok())
}

fn write_to_database(
    database: &Arc<redb::Database>,
    key: &str,
    payload: &[u8],
    expires_at_ms: u64,
) -> Result<(), RedbCacheError> {
    let encoded = encode_entry(expires_at_ms, payload);
    let tx = database.begin_write()?;
    {
        let mut table = tx.open_table(TABLE_DEFINITION)?;
        table.insert(key, encoded.as_slice())?;
    }
    tx.commit()?;
    Ok(())
}

fn remove_expired(database: &Arc<redb::Database>, key: &str) -> Result<(), RedbCacheError> {
    let tx = database.begin_write()?;
    {
        let mut table = tx.open_table(TABLE_DEFINITION)?;
        let _ = table.remove(key)?;
    }
    tx.commit()?;
    Ok(())
}

fn get_redb_state(state: &Rc<RefCell<OpState>>) -> Result<Arc<RedbCacheState>, RedbCacheError> {
    state.borrow().try_borrow::<Arc<RedbCacheState>>().cloned().ok_or(RedbCacheError::StateMissing)
}

async fn run_redb<T, F>(f: F) -> Result<T, JsErrorBox>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, RedbCacheError> + Send + 'static,
{
    spawn_blocking(f).await.map_err(|e| js_error(&e))?.map_err(|e| js_error(&e))
}

#[op2]
#[string]
pub async fn op_redb_cache_get(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
) -> Result<Option<String>, JsErrorBox> {
    let redb_state = get_redb_state(&state).map_err(|e| js_error(&e))?;

    run_redb(move || {
        let database = redb_state.database()?;
        read_from_database(&database, &key)
    })
    .await
}

#[op2]
pub async fn op_redb_cache_set(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
    #[string] value: String,
    #[smi] ttl_ms: u32,
) -> Result<(), JsErrorBox> {
    let redb_state = get_redb_state(&state).map_err(|e| js_error(&e))?;
    let default_ttl_secs = redb_state.default_ttl_secs;
    let payload = value.into_bytes();
    let expires_at_ms =
        expires_at_ms_from_ttl(ttl_ms, default_ttl_secs).map_err(|e| js_error(&e))?;

    run_redb(move || {
        let database = redb_state.database()?;
        write_to_database(&database, &key, &payload, expires_at_ms)
    })
    .await
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use std::{fs, thread::sleep, time::Duration};

    use redb::ReadableTable;

    use super::*;

    fn test_db_path(test_name: &str) -> PathBuf {
        env::temp_dir().join(format!("rari-test-redb-cache-{test_name}.redb"))
    }

    fn open_temp_db(test_name: &str) -> Arc<redb::Database> {
        let path = test_db_path(test_name);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        Arc::new(redb::Database::create(&path).expect("create db"))
    }

    fn cleanup_expired_all(database: &Arc<redb::Database>) -> Result<usize, RedbCacheError> {
        let now = now_ms()?;
        let mut expired_keys: Vec<String> = Vec::new();
        {
            let tx = database.begin_read()?;
            if let Ok(table) = tx.open_table(TABLE_DEFINITION) {
                for entry in table.iter()? {
                    let (key, value) = entry?;
                    let Some((expires_at_ms, _)) = decode_entry(value.value()) else {
                        continue;
                    };
                    if expires_at_ms != EXPIRY_NEVER && now >= expires_at_ms {
                        expired_keys.push(key.value().to_string());
                    }
                }
            }
        }

        if expired_keys.is_empty() {
            return Ok(0);
        }

        let removed = expired_keys.len();
        let tx = database.begin_write()?;
        {
            let mut table = tx.open_table(TABLE_DEFINITION)?;
            for key in &expired_keys {
                let _ = table.remove(key.as_str())?;
            }
        }
        tx.commit()?;
        Ok(removed)
    }

    #[test]
    fn expires_at_ms_from_ttl_default_when_zero() {
        let now = now_ms().expect("system time");
        let expires = expires_at_ms_from_ttl(0, 60).expect("ttl");
        assert!(expires >= now + 59_000);
        assert!(expires <= now + 61_000);
    }

    #[test]
    fn expires_at_ms_from_ttl_never_when_default_zero() {
        let expires = expires_at_ms_from_ttl(0, 0).expect("ttl");
        assert_eq!(expires, EXPIRY_NEVER);
    }

    #[test]
    fn expires_at_ms_from_ttl_explicit_ttl() {
        let now = now_ms().expect("system time");
        let expires = expires_at_ms_from_ttl(5_000, 60).expect("ttl");
        assert!(expires >= now + 4_500);
        assert!(expires <= now + 5_500);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let expires: u64 = 123_456_789;
        let payload = b"hello world";
        let encoded = encode_entry(expires, payload);
        let (got_expires, got_payload) = decode_entry(&encoded).expect("decoded");
        assert_eq!(got_expires, expires);
        assert_eq!(got_payload, payload);
    }

    #[test]
    fn decode_too_short_returns_none() {
        assert!(decode_entry(&[1, 2, 3]).is_none());
        assert!(decode_entry(&[]).is_none());
    }

    #[test]
    fn write_then_read_roundtrip() {
        let db = open_temp_db("write-then-read");
        write_to_database(&db, "k1", b"hello", EXPIRY_NEVER).expect("write");
        let got = read_from_database(&db, "k1").expect("read");
        assert_eq!(got.as_deref(), Some("hello"));
    }

    #[test]
    fn read_missing_key_returns_none() {
        let db = open_temp_db("read-missing");
        let got = read_from_database(&db, "absent").expect("read");
        assert!(got.is_none());
    }

    #[test]
    fn expired_entry_returns_none_and_is_removed() {
        let db = open_temp_db("expired-entry");
        let expires_at = now_ms().expect("now").saturating_add(50);
        write_to_database(&db, "k", b"x", expires_at).expect("write");

        sleep(Duration::from_millis(120));

        let got = read_from_database(&db, "k").expect("read");
        assert!(got.is_none());

        let tx = db.begin_read().expect("tx");
        let table = tx.open_table(TABLE_DEFINITION).expect("table");
        assert!(table.get("k").expect("get").is_none());
    }

    #[test]
    fn never_expires_entry_persists_past_ttl_window() {
        let db = open_temp_db("never-expires");
        write_to_database(&db, "k", b"x", EXPIRY_NEVER).expect("write");
        sleep(Duration::from_millis(80));
        let got = read_from_database(&db, "k").expect("read");
        assert_eq!(got.as_deref(), Some("x"));
    }

    #[test]
    fn cleanup_expired_all_removes_only_expired() {
        let db = open_temp_db("cleanup-expired");
        let now = now_ms().expect("now");
        write_to_database(&db, "expired1", b"a", now.saturating_sub(10)).expect("write 1");
        write_to_database(&db, "expired2", b"b", now.saturating_sub(5)).expect("write 2");
        write_to_database(&db, "fresh", b"c", now.saturating_add(60_000)).expect("write 3");
        write_to_database(&db, "forever", b"d", EXPIRY_NEVER).expect("write 4");

        let removed = cleanup_expired_all(&db).expect("cleanup");
        assert_eq!(removed, 2);

        let tx = db.begin_read().expect("tx");
        let table = tx.open_table(TABLE_DEFINITION).expect("table");
        assert!(table.get("expired1").expect("get").is_none());
        assert!(table.get("expired2").expect("get").is_none());
        assert!(table.get("fresh").expect("get").is_some());
        assert!(table.get("forever").expect("get").is_some());
    }
}
