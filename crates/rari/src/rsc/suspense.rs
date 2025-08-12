use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspenseBoundary {
    pub id: String,
    pub fallback: serde_json::Value,
    pub parent_id: Option<String>,
    pub children_boundaries: Vec<String>,
    pub pending_promises: Vec<String>,
    pub resolved: bool,
    pub error: Option<String>,
    pub created_at: u64,
    pub component_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseInfo {
    pub id: String,
    pub component_id: String,
    pub boundary_id: String,
    pub status: PromiseStatus,
    pub resolved_value: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cache_key: Option<String>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromiseStatus {
    Pending,
    Resolved,
    Rejected,
    Cached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspenseError {
    pub message: String,
    pub boundary_id: String,
    pub component_id: String,
    pub stack: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspenseBoundaryState {
    pub resolved: bool,
    pub pending_count: usize,
    pub error: Option<String>,
    pub fallback: serde_json::Value,
}

#[derive(Debug)]
pub struct SuspenseManager {
    boundaries: FxHashMap<String, SuspenseBoundary>,
    promise_cache: FxHashMap<String, PromiseInfo>,
    boundary_stack: Vec<String>,
    global_promise_counter: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspenseStats {
    pub total_boundaries: usize,
    pub pending_boundaries: usize,
    pub resolved_boundaries: usize,
    pub error_boundaries: usize,
    pub total_promises: usize,
    pub pending_promises: usize,
    pub resolved_promises: usize,
    pub rejected_promises: usize,
    pub cached_promises: usize,
    pub boundary_stack_depth: usize,
}

impl SuspenseManager {
    pub fn new() -> Self {
        Self {
            boundaries: FxHashMap::default(),
            promise_cache: FxHashMap::default(),
            boundary_stack: Vec::new(),
            global_promise_counter: 0,
        }
    }

    pub fn create_boundary(
        &mut self,
        id: String,
        fallback: serde_json::Value,
        component_path: String,
    ) -> String {
        let boundary = SuspenseBoundary {
            id: id.clone(),
            fallback,
            parent_id: self.boundary_stack.last().cloned(),
            children_boundaries: Vec::new(),
            pending_promises: Vec::new(),
            resolved: false,
            error: None,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                as u64,
            component_path,
        };

        self.boundaries.insert(id.clone(), boundary);
        self.boundary_stack.push(id.clone());
        id
    }

    pub fn register_promise(
        &mut self,
        component_id: String,
        boundary_id: String,
        cache_key: Option<String>,
    ) -> String {
        self.global_promise_counter += 1;
        let promise_id = format!("promise_{}", self.global_promise_counter);

        let promise_info = PromiseInfo {
            id: promise_id.clone(),
            component_id,
            boundary_id: boundary_id.clone(),
            status: PromiseStatus::Pending,
            resolved_value: None,
            error: None,
            cache_key,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                as u64,
            resolved_at: None,
        };

        self.promise_cache.insert(promise_id.clone(), promise_info);

        if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
            boundary.pending_promises.push(promise_id.clone());
        }

        promise_id
    }

    pub fn resolve_promise(&mut self, promise_id: &str, value: serde_json::Value) -> bool {
        if let Some(promise) = self.promise_cache.get_mut(promise_id) {
            promise.status = PromiseStatus::Resolved;
            promise.resolved_value = Some(value);
            promise.resolved_at =
                Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                    as u64);

            if let Some(boundary) = self.boundaries.get_mut(&promise.boundary_id) {
                boundary.pending_promises.retain(|p| p != promise_id);
                if boundary.pending_promises.is_empty() {
                    boundary.resolved = true;
                }
            }

            true
        } else {
            false
        }
    }

    pub fn reject_promise(&mut self, promise_id: &str, error: String) -> bool {
        if let Some(promise) = self.promise_cache.get_mut(promise_id) {
            promise.status = PromiseStatus::Rejected;
            promise.error = Some(error.clone());
            promise.resolved_at =
                Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                    as u64);

            if let Some(boundary) = self.boundaries.get_mut(&promise.boundary_id) {
                boundary.error = Some(error);
                boundary.pending_promises.retain(|p| p != promise_id);
            }

            true
        } else {
            false
        }
    }

    pub fn pop_boundary(&mut self) -> Option<String> {
        self.boundary_stack.pop()
    }

    pub fn get_boundary_state(&self, boundary_id: &str) -> Option<SuspenseBoundaryState> {
        self.boundaries.get(boundary_id).map(|boundary| SuspenseBoundaryState {
            resolved: boundary.resolved,
            pending_count: boundary.pending_promises.len(),
            error: boundary.error.clone(),
            fallback: boundary.fallback.clone(),
        })
    }

    pub fn get_resolved_boundaries(&self) -> Vec<String> {
        self.boundaries
            .iter()
            .filter(|(_, boundary)| boundary.resolved)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_promise_info(&self, promise_id: &str) -> Option<&PromiseInfo> {
        self.promise_cache.get(promise_id)
    }

    pub fn get_cached_promise(&self, cache_key: &str) -> Option<&PromiseInfo> {
        self.promise_cache.values().find(|p| {
            p.cache_key.as_deref() == Some(cache_key) && p.status == PromiseStatus::Cached
        })
    }

    pub fn cleanup_resolved(&mut self) {
        self.boundaries.retain(|_, boundary| !boundary.resolved);

        self.promise_cache.retain(|_, promise| {
            matches!(promise.status, PromiseStatus::Pending | PromiseStatus::Cached)
        });

        self.boundary_stack.retain(|id| self.boundaries.contains_key(id));
    }

    pub fn get_boundary_depth(&self) -> usize {
        self.boundary_stack.len()
    }

    pub fn has_pending_promises(&self) -> bool {
        self.promise_cache.values().any(|p| p.status == PromiseStatus::Pending)
    }

    pub fn get_pending_promises(&self) -> Vec<&PromiseInfo> {
        self.promise_cache.values().filter(|p| p.status == PromiseStatus::Pending).collect()
    }

    pub fn set_boundary_error(&mut self, boundary_id: &str, error: String) {
        if let Some(boundary) = self.boundaries.get_mut(boundary_id) {
            boundary.error = Some(error);
        }
    }

    pub fn get_current_boundary(&self) -> Option<&String> {
        self.boundary_stack.last()
    }

    pub fn has_pending_boundaries(&self) -> bool {
        self.boundaries.values().any(|b| !b.resolved && b.error.is_none())
    }

    pub fn get_stats(&self) -> SuspenseStats {
        let total_boundaries = self.boundaries.len();
        let pending_boundaries =
            self.boundaries.values().filter(|b| !b.resolved && b.error.is_none()).count();
        let resolved_boundaries = self.boundaries.values().filter(|b| b.resolved).count();
        let error_boundaries = self.boundaries.values().filter(|b| b.error.is_some()).count();

        let total_promises = self.promise_cache.len();
        let pending_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Pending).count();
        let resolved_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Resolved).count();
        let rejected_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Rejected).count();
        let cached_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Cached).count();

        SuspenseStats {
            total_boundaries,
            pending_boundaries,
            resolved_boundaries,
            error_boundaries,
            total_promises,
            pending_promises,
            resolved_promises,
            rejected_promises,
            cached_promises,
            boundary_stack_depth: self.boundary_stack.len(),
        }
    }

    pub fn boundaries_count(&self) -> usize {
        self.boundaries.len()
    }

    pub fn promise_cache_count(&self) -> usize {
        self.promise_cache.len()
    }
}

impl Default for SuspenseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspenseBoundary {
    pub fn new(id: String, fallback: serde_json::Value, component_path: String) -> Self {
        Self {
            id,
            fallback,
            parent_id: None,
            children_boundaries: Vec::new(),
            pending_promises: Vec::new(),
            resolved: false,
            error: None,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                as u64,
            component_path,
        }
    }

    pub fn is_ready_to_resolve(&self) -> bool {
        self.pending_promises.is_empty() && self.error.is_none()
    }

    pub fn get_age_seconds(&self) -> u64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let created_seconds = self.created_at / 1000;
        now.saturating_sub(created_seconds)
    }
}

impl PromiseInfo {
    pub fn new(
        id: String,
        component_id: String,
        boundary_id: String,
        cache_key: Option<String>,
    ) -> Self {
        Self {
            id,
            component_id,
            boundary_id,
            status: PromiseStatus::Pending,
            resolved_value: None,
            error: None,
            cache_key,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
                as u64,
            resolved_at: None,
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.status == PromiseStatus::Resolved
    }

    pub fn is_rejected(&self) -> bool {
        self.status == PromiseStatus::Rejected
    }

    pub fn get_age_ms(&self) -> u64 {
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        now.saturating_sub(self.created_at)
    }
}

impl SuspenseError {
    pub fn new(message: String, boundary_id: String, component_id: String) -> Self {
        Self { message, boundary_id, component_id, stack: None, digest: None }
    }

    pub fn with_stack(mut self, stack: String) -> Self {
        self.stack = Some(stack);
        self
    }

    pub fn with_digest(mut self, digest: String) -> Self {
        self.digest = Some(digest);
        self
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn create_test_fallback() -> serde_json::Value {
        serde_json::json!({
            "type": "div",
            "props": {}
        })
    }

    #[test]
    fn test_suspense_manager_creation() {
        let manager = SuspenseManager::new();
        assert_eq!(manager.boundaries_count(), 0);
        assert_eq!(manager.promise_cache_count(), 0);
        assert_eq!(manager.get_boundary_depth(), 0);
    }

    #[test]
    fn test_create_boundary() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let boundary_id = manager.create_boundary(
            "test-boundary".to_string(),
            fallback.clone(),
            "TestComponent".to_string(),
        );

        assert_eq!(boundary_id, "test-boundary");
        assert_eq!(manager.boundaries_count(), 1);
        assert_eq!(manager.get_boundary_depth(), 1);

        let state = manager.get_boundary_state(&boundary_id).unwrap();
        assert!(!state.resolved);
        assert_eq!(state.pending_count, 0);
        assert_eq!(state.fallback, fallback);
    }

    #[test]
    fn test_register_promise() {
        let mut manager = SuspenseManager::new();
        let boundary_id = manager.create_boundary(
            "test-boundary".to_string(),
            create_test_fallback(),
            "TestComponent".to_string(),
        );

        let promise_id = manager.register_promise(
            "TestComponent".to_string(),
            boundary_id.clone(),
            Some("cache-key".to_string()),
        );

        assert_eq!(manager.promise_cache_count(), 1);
        assert!(manager.has_pending_promises());

        let promise_info = manager.get_promise_info(&promise_id).unwrap();
        assert_eq!(promise_info.status, PromiseStatus::Pending);
        assert_eq!(promise_info.boundary_id, boundary_id);

        let state = manager.get_boundary_state(&boundary_id).unwrap();
        assert_eq!(state.pending_count, 1);
        assert!(!state.resolved);
    }
}
