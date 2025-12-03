use crate::runtime::module_reload::ModuleReloadRequest;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DebouncePendingMap = FxHashMap<String, (ModuleReloadRequest, tokio::task::JoinHandle<()>)>;

#[derive(Debug)]
pub struct DebounceManager {
    pending: Arc<Mutex<DebouncePendingMap>>,
}

impl DebounceManager {
    pub fn new() -> Self {
        Self { pending: Arc::new(Mutex::new(FxHashMap::default())) }
    }

    pub fn pending(&self) -> &Arc<Mutex<DebouncePendingMap>> {
        &self.pending
    }

    pub async fn cancel_pending(&self, component_id: &str) -> bool {
        let mut pending = self.pending.lock().await;
        if let Some((_, handle)) = pending.remove(component_id) {
            handle.abort();
            true
        } else {
            false
        }
    }

    pub async fn add_pending(
        &self,
        component_id: String,
        request: ModuleReloadRequest,
        handle: tokio::task::JoinHandle<()>,
    ) {
        let mut pending = self.pending.lock().await;
        pending.insert(component_id, (request, handle));
    }

    pub async fn remove_pending(&self, component_id: &str) -> Option<ModuleReloadRequest> {
        let mut pending = self.pending.lock().await;
        pending.remove(component_id).map(|(request, _)| request)
    }
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending.lock().await;
        pending.len()
    }

    pub async fn clear_all(&self) {
        let mut pending = self.pending.lock().await;
        for (_, (_, handle)) in pending.drain() {
            handle.abort();
        }
    }

    pub async fn has_pending(&self, component_id: &str) -> bool {
        let pending = self.pending.lock().await;
        pending.contains_key(component_id)
    }
}

impl Default for DebounceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DebounceManager {
    fn clone(&self) -> Self {
        Self { pending: Arc::clone(&self.pending) }
    }
}
