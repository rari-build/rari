use crate::error::RariError;
use crate::runtime::module_loader::config::BatchStats;
use crate::runtime::module_loader::interner::get_string_interner;
use dashmap::DashMap;
use parking_lot::Mutex;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

pub type ModuleOperations = SmallVec<[LoaderModuleOperation; 8]>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum StorageKey {
    ModuleCode(String),
    ModuleMeta(String),
    Version(String),
}

#[derive(Debug, Clone)]
enum StorageValue {
    Code(String),
    Meta(bool),
    Version(u64),
}

impl StorageValue {
    fn as_code(&self) -> Option<&String> {
        match self {
            StorageValue::Code(code) => Some(code),
            _ => None,
        }
    }

    fn as_meta(&self) -> Option<bool> {
        match self {
            StorageValue::Meta(meta) => Some(*meta),
            _ => None,
        }
    }

    fn as_version(&self) -> Option<u64> {
        match self {
            StorageValue::Version(version) => Some(*version),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct BatchedOperation {
    operations: ModuleOperations,
    created_at: Instant,
}

#[derive(Debug)]
pub enum LoaderModuleOperation {
    AddModule { specifier: Arc<str>, code: Arc<str> },
}

#[derive(Debug)]
pub struct OrderedStorage {
    storage: DashMap<StorageKey, StorageValue>,
    pending_batch: Arc<Mutex<Option<BatchedOperation>>>,
    batch_size_limit: usize,
    batch_time_limit: Duration,
    total_batches_processed: AtomicUsize,
    total_operations_batched: AtomicUsize,
    batch_flush_failures: AtomicUsize,
    total_batch_time_saved_ms: AtomicU64,
}

impl OrderedStorage {
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
            pending_batch: Arc::new(Mutex::new(None)),
            batch_size_limit: 50,
            batch_time_limit: Duration::from_millis(100),
            total_batches_processed: AtomicUsize::new(0),
            total_operations_batched: AtomicUsize::new(0),
            batch_flush_failures: AtomicUsize::new(0),
            total_batch_time_saved_ms: AtomicU64::new(0),
        }
    }

    pub fn get_module_code(&self, specifier: &str) -> Option<String> {
        self.storage
            .get(&StorageKey::ModuleCode(specifier.to_string()))
            .and_then(|entry| entry.value().as_code().cloned())
    }

    pub fn get_module_meta(&self, specifier: &str) -> Option<bool> {
        self.storage
            .get(&StorageKey::ModuleMeta(specifier.to_string()))
            .and_then(|entry| entry.value().as_meta())
    }

    pub fn get_version(&self, specifier: &str) -> Option<u64> {
        self.storage
            .get(&StorageKey::Version(specifier.to_string()))
            .and_then(|entry| entry.value().as_version())
    }

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.storage.insert(StorageKey::ModuleCode(specifier), StorageValue::Code(code));
    }

    pub fn set_module_meta(&self, specifier: String, meta: bool) {
        self.storage.insert(StorageKey::ModuleMeta(specifier), StorageValue::Meta(meta));
    }

    pub fn set_version(&self, specifier: String, version: u64) {
        self.storage.insert(StorageKey::Version(specifier), StorageValue::Version(version));
    }

    pub fn contains_module_code(&self, specifier: &str) -> bool {
        self.storage.contains_key(&StorageKey::ModuleCode(specifier.to_string()))
    }

    fn add_to_batch(&self, operation: LoaderModuleOperation) -> Result<(), RariError> {
        let now = Instant::now();

        let mut pending = self.pending_batch.lock();

        let should_flush = if let Some(ref mut batch) = pending.as_mut() {
            batch.operations.push(operation);

            batch.operations.len() >= self.batch_size_limit
                || now.duration_since(batch.created_at) >= self.batch_time_limit
        } else {
            *pending = Some(BatchedOperation { operations: smallvec![operation], created_at: now });
            false
        };

        if should_flush {
            let batch = pending.take().expect("Batch should exist when should_flush is true");
            drop(pending);
            self.flush_batch(batch)?;
        }

        Ok(())
    }

    pub fn flush_pending_batch(&self) -> Result<(), RariError> {
        let mut pending = self.pending_batch.lock();

        if let Some(batch) = pending.take() {
            drop(pending);
            self.flush_batch(batch)?;
        }

        Ok(())
    }

    fn flush_batch(&self, batch: BatchedOperation) -> Result<(), RariError> {
        if batch.operations.is_empty() {
            return Ok(());
        }

        let operation_count = batch.operations.len();

        for operation in &batch.operations {
            match operation {
                LoaderModuleOperation::AddModule { specifier, code } => {
                    self.set_module_code(specifier.to_string(), code.to_string());
                }
            }
        }

        self.total_batches_processed.fetch_add(1, Ordering::Relaxed);
        self.total_operations_batched.fetch_add(operation_count, Ordering::Relaxed);

        let time_saved_estimate = (operation_count.saturating_sub(1)) * 2;
        self.total_batch_time_saved_ms.fetch_add(time_saved_estimate as u64, Ordering::Relaxed);

        Ok(())
    }

    pub fn add_module_interned(&self, specifier: &str, code: &str) -> Result<(), RariError> {
        let interner = get_string_interner();

        let operation = LoaderModuleOperation::AddModule {
            specifier: interner.intern(specifier),
            code: interner.intern(code),
        };

        self.add_to_batch(operation)
    }

    pub fn get_batch_stats(&self) -> BatchStats {
        let total_batches = self.total_batches_processed.load(Ordering::Relaxed);
        let total_ops = self.total_operations_batched.load(Ordering::Relaxed);

        BatchStats {
            total_batches_processed: total_batches,
            total_operations_batched: total_ops,
            average_batch_size: if total_batches > 0 {
                total_ops as f64 / total_batches as f64
            } else {
                0.0
            },
            batch_flush_failures: self.batch_flush_failures.load(Ordering::Relaxed),
            time_saved_by_batching_ms: self.total_batch_time_saved_ms.load(Ordering::Relaxed),
        }
    }
}

impl Default for OrderedStorage {
    fn default() -> Self {
        Self::new()
    }
}
