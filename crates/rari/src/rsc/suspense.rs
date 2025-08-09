use crate::rsc::serializer::ReactElement;
use deno_core::anyhow::{Result, anyhow};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct SuspenseBoundary {
    pub id: String,
    pub fallback: ReactElement,
    pub parent_id: Option<String>,
    pub children_boundaries: Vec<String>,
    pub pending_promises: Vec<String>,
    pub resolved: bool,
    pub error: Option<SuspenseError>,
    pub created_at: SystemTime,
    pub component_path: String,
}

#[derive(Debug, Clone)]
pub struct PromiseInfo {
    pub id: String,
    pub component_id: String,
    pub boundary_id: String,
    pub status: PromiseStatus,
    pub resolved_value: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cache_key: Option<String>,
    pub created_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromiseStatus {
    Pending,
    Resolved,
    Rejected,
    Cached,
}

#[derive(Debug, Clone)]
pub struct SuspenseError {
    pub message: String,
    pub boundary_id: String,
    pub component_id: String,
    pub stack: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SuspenseBoundaryState {
    pub resolved: bool,
    pub pending_count: usize,
    pub error: Option<SuspenseError>,
    pub fallback: ReactElement,
}

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
        fallback: ReactElement,
        parent_id: Option<String>,
        component_path: String,
    ) -> Result<String> {
        if self.boundaries.contains_key(&id) {
            return Err(anyhow!("Suspense boundary with id '{}' already exists", id));
        }

        if let Some(ref parent) = parent_id {
            if !self.boundaries.contains_key(parent) {
                return Err(anyhow!("Parent boundary '{}' not found", parent));
            }

            if let Some(parent_boundary) = self.boundaries.get_mut(parent) {
                parent_boundary.children_boundaries.push(id.clone());
            }
        }

        let boundary = SuspenseBoundary {
            id: id.clone(),
            fallback,
            parent_id,
            children_boundaries: Vec::new(),
            pending_promises: Vec::new(),
            resolved: false,
            error: None,
            created_at: SystemTime::now(),
            component_path,
        };

        self.boundaries.insert(id.clone(), boundary);
        self.boundary_stack.push(id.clone());

        Ok(id)
    }

    pub fn register_promise(
        &mut self,
        component_id: String,
        boundary_id: String,
        cache_key: Option<String>,
    ) -> Result<String> {
        self.global_promise_counter += 1;
        let promise_id = format!("promise-{}", self.global_promise_counter);

        if !self.boundaries.contains_key(&boundary_id) {
            return Err(anyhow!("Boundary '{}' not found", boundary_id));
        }

        if let Some(cache_key) = &cache_key {
            for promise in self.promise_cache.values() {
                if promise.cache_key.as_ref() == Some(cache_key)
                    && promise.status == PromiseStatus::Resolved
                {
                    return Ok(promise.id.clone());
                }
            }
        }

        let promise_info = PromiseInfo {
            id: promise_id.clone(),
            component_id,
            boundary_id: boundary_id.clone(),
            status: PromiseStatus::Pending,
            resolved_value: None,
            error: None,
            cache_key,
            created_at: SystemTime::now(),
            resolved_at: None,
        };

        self.promise_cache.insert(promise_id.clone(), promise_info);

        if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
            boundary.pending_promises.push(promise_id.clone());
        }

        Ok(promise_id)
    }

    pub fn resolve_promise(
        &mut self,
        promise_id: String,
        resolved_value: serde_json::Value,
    ) -> Result<Vec<String>> {
        let mut affected_boundaries = Vec::new();

        if let Some(promise) = self.promise_cache.get_mut(&promise_id) {
            promise.status = PromiseStatus::Resolved;
            promise.resolved_value = Some(resolved_value);
            promise.resolved_at = Some(SystemTime::now());

            let boundary_id = promise.boundary_id.clone();

            if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
                boundary.pending_promises.retain(|id| id != &promise_id);

                if boundary.pending_promises.is_empty() {
                    boundary.resolved = true;
                    affected_boundaries.push(boundary_id.clone());

                    let mut current_parent = boundary.parent_id.clone();
                    while let Some(parent_id) = current_parent {
                        let should_resolve_parent = {
                            if let Some(parent_boundary) = self.boundaries.get(&parent_id) {
                                let all_children_resolved =
                                    parent_boundary.children_boundaries.iter().all(|child_id| {
                                        self.boundaries
                                            .get(child_id)
                                            .map(|b| b.resolved)
                                            .unwrap_or(false)
                                    });

                                all_children_resolved && parent_boundary.pending_promises.is_empty()
                            } else {
                                false
                            }
                        };

                        if should_resolve_parent {
                            let parent_boundary_parent =
                                self.boundaries.get(&parent_id).and_then(|b| b.parent_id.clone());
                            if let Some(parent_boundary) = self.boundaries.get_mut(&parent_id) {
                                parent_boundary.resolved = true;
                                affected_boundaries.push(parent_id.clone());
                                current_parent = parent_boundary_parent;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        } else {
            return Err(anyhow!("Promise '{}' not found", promise_id));
        }

        Ok(affected_boundaries)
    }

    pub fn reject_promise(
        &mut self,
        promise_id: String,
        error_message: String,
    ) -> Result<Vec<String>> {
        let mut affected_boundaries = Vec::new();

        if let Some(promise) = self.promise_cache.get_mut(&promise_id) {
            promise.status = PromiseStatus::Rejected;
            promise.error = Some(error_message.clone());
            promise.resolved_at = Some(SystemTime::now());

            let boundary_id = promise.boundary_id.clone();
            let component_id = promise.component_id.clone();

            let suspense_error = SuspenseError {
                message: error_message,
                boundary_id: boundary_id.clone(),
                component_id,
                stack: None,
                digest: None,
            };

            if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
                boundary.error = Some(suspense_error);
                boundary.pending_promises.retain(|id| id != &promise_id);
                affected_boundaries.push(boundary_id);
            }
        } else {
            return Err(anyhow!("Promise '{}' not found", promise_id));
        }

        Ok(affected_boundaries)
    }

    fn _get_nearest_boundary(&self, _component_path: &str) -> Option<String> {
        self.boundary_stack.last().cloned()
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

    pub fn get_resolved_boundaries(&self) -> Vec<&SuspenseBoundary> {
        self.boundaries
            .values()
            .filter(|boundary| boundary.resolved && boundary.error.is_none())
            .collect()
    }

    pub fn get_promise_info(&self, promise_id: &str) -> Option<&PromiseInfo> {
        self.promise_cache.get(promise_id)
    }

    pub fn get_cached_promise(&self, cache_key: &str) -> Option<&PromiseInfo> {
        self.promise_cache.values().find(|promise| {
            promise.cache_key.as_ref() == Some(&cache_key.to_string())
                && promise.status == PromiseStatus::Resolved
        })
    }

    pub fn cleanup_resolved(&mut self, max_age_seconds: u64) -> usize {
        let cutoff_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs()
            .saturating_sub(max_age_seconds);

        let mut cleaned_count = 0;

        let promise_ids_to_remove: Vec<String> = self
            .promise_cache
            .iter()
            .filter_map(|(id, promise)| {
                if let Some(resolved_at) = promise.resolved_at
                    && resolved_at
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                        <= cutoff_time
                    && (promise.status == PromiseStatus::Resolved
                        || promise.status == PromiseStatus::Rejected)
                {
                    return Some(id.clone());
                }
                None
            })
            .collect();

        for promise_id in promise_ids_to_remove {
            if let Some(promise) = self.promise_cache.remove(&promise_id) {
                if let Some(boundary) = self.boundaries.get_mut(&promise.boundary_id) {
                    boundary.pending_promises.retain(|id| id != &promise_id);
                }
                cleaned_count += 1;
            }
        }

        let boundary_ids_to_remove: Vec<String> = self
            .boundaries
            .iter()
            .filter_map(|(id, boundary)| {
                if boundary.resolved
                    && boundary.pending_promises.is_empty()
                    && boundary.children_boundaries.is_empty()
                    && boundary
                        .created_at
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                        <= cutoff_time
                {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        for boundary_id in boundary_ids_to_remove {
            if self.boundaries.remove(&boundary_id).is_some() {
                cleaned_count += 1;
            }
        }

        cleaned_count
    }

    pub fn get_boundary_depth(&self) -> usize {
        self.boundary_stack.len()
    }

    pub fn has_pending_promises(&self, boundary_id: &str) -> bool {
        self.boundaries
            .get(boundary_id)
            .map(|boundary| !boundary.pending_promises.is_empty())
            .unwrap_or(false)
    }

    pub fn get_pending_promises(&self, boundary_id: &str) -> Vec<String> {
        self.boundaries
            .get(boundary_id)
            .map(|boundary| boundary.pending_promises.clone())
            .unwrap_or_default()
    }

    pub fn set_boundary_error(&mut self, boundary_id: String, error: SuspenseError) -> Result<()> {
        if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
            boundary.error = Some(error);
            Ok(())
        } else {
            Err(anyhow!("Boundary '{}' not found", boundary_id))
        }
    }

    pub fn get_current_boundary(&self) -> Option<&String> {
        self.boundary_stack.last()
    }

    pub fn has_pending_boundaries(&self) -> bool {
        self.boundaries.values().any(|boundary| !boundary.resolved && boundary.error.is_none())
    }

    pub fn get_stats(&self) -> SuspenseStats {
        let total_boundaries = self.boundaries.len();
        let resolved_boundaries = self.boundaries.values().filter(|b| b.resolved).count();
        let pending_boundaries =
            self.boundaries.values().filter(|b| !b.resolved && b.error.is_none()).count();
        let error_boundaries = self.boundaries.values().filter(|b| b.error.is_some()).count();

        let total_promises = self.promise_cache.len();
        let resolved_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Resolved).count();
        let pending_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Pending).count();
        let rejected_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Rejected).count();
        let cached_promises =
            self.promise_cache.values().filter(|p| p.status == PromiseStatus::Cached).count();

        SuspenseStats {
            total_boundaries,
            resolved_boundaries,
            pending_boundaries,
            error_boundaries,
            total_promises,
            resolved_promises,
            pending_promises,
            rejected_promises,
            cached_promises,
            boundary_stack_depth: self.boundary_stack.len(),
        }
    }

    pub fn handle_boundary_error(
        &mut self,
        boundary_id: String,
        error: SuspenseError,
        recovery_strategy: SuspenseRecoveryStrategy,
    ) -> Result<SuspenseRecoveryAction> {
        if let Some(boundary) = self.boundaries.get_mut(&boundary_id) {
            boundary.error = Some(error.clone());
        }

        match recovery_strategy {
            SuspenseRecoveryStrategy::RetryWithBackoff { max_attempts, delay_ms } => {
                self.schedule_retry(boundary_id, max_attempts, delay_ms)
            }
            SuspenseRecoveryStrategy::FallbackToErrorBoundary => {
                Ok(SuspenseRecoveryAction::ShowErrorBoundary(error))
            }
            SuspenseRecoveryStrategy::PropagateToParent => {
                self.propagate_error_to_parent(boundary_id, error)
            }
            SuspenseRecoveryStrategy::ShowErrorUI => {
                Ok(SuspenseRecoveryAction::ShowErrorUI(error.message))
            }
        }
    }

    fn schedule_retry(
        &mut self,
        boundary_id: String,
        max_attempts: u32,
        base_delay_ms: u64,
    ) -> Result<SuspenseRecoveryAction> {
        Ok(SuspenseRecoveryAction::ScheduleRetry {
            boundary_id,
            attempts_remaining: max_attempts,
            next_delay_ms: base_delay_ms,
        })
    }

    fn propagate_error_to_parent(
        &mut self,
        boundary_id: String,
        error: SuspenseError,
    ) -> Result<SuspenseRecoveryAction> {
        let parent_id =
            self.boundaries.get(&boundary_id).and_then(|boundary| boundary.parent_id.clone());

        if let Some(parent_id) = parent_id {
            let parent_error = SuspenseError {
                message: format!("Child boundary error: {}", error.message),
                boundary_id: parent_id.clone(),
                component_id: error.component_id,
                stack: error.stack,
                digest: error.digest,
            };

            if let Some(parent_boundary) = self.boundaries.get_mut(&parent_id) {
                parent_boundary.error = Some(parent_error.clone());
            }

            return Ok(SuspenseRecoveryAction::PropagateError {
                parent_boundary_id: parent_id.clone(),
                error: parent_error,
            });
        }

        Ok(SuspenseRecoveryAction::ShowErrorUI(error.message))
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

#[derive(Debug, Clone)]
pub enum SuspenseRecoveryStrategy {
    RetryWithBackoff { max_attempts: u32, delay_ms: u64 },
    FallbackToErrorBoundary,
    PropagateToParent,
    ShowErrorUI,
}

#[derive(Debug, Clone)]
pub enum SuspenseRecoveryAction {
    ScheduleRetry { boundary_id: String, attempts_remaining: u32, next_delay_ms: u64 },
    ShowErrorBoundary(SuspenseError),
    PropagateError { parent_boundary_id: String, error: SuspenseError },
    ShowErrorUI(String),
}

impl SuspenseBoundary {
    pub fn new(
        id: String,
        fallback: ReactElement,
        parent_id: Option<String>,
        component_path: String,
    ) -> Self {
        Self {
            id,
            fallback,
            parent_id,
            children_boundaries: Vec::new(),
            pending_promises: Vec::new(),
            resolved: false,
            error: None,
            created_at: SystemTime::now(),
            component_path,
        }
    }

    pub fn is_ready_to_resolve(&self) -> bool {
        self.pending_promises.is_empty() && self.error.is_none()
    }

    pub fn get_age_seconds(&self) -> u64 {
        SystemTime::now().duration_since(self.created_at).unwrap_or_default().as_secs()
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
            created_at: SystemTime::now(),
            resolved_at: None,
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.status, PromiseStatus::Resolved | PromiseStatus::Cached)
    }

    pub fn is_rejected(&self) -> bool {
        self.status == PromiseStatus::Rejected
    }

    pub fn get_age_ms(&self) -> u128 {
        SystemTime::now().duration_since(self.created_at).unwrap_or_default().as_millis()
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
mod tests {
    use super::*;
    use crate::rsc::serializer::ElementType;

    fn create_test_fallback() -> ReactElement {
        ReactElement {
            element_type: ElementType::Text("Loading...".to_string()),
            props: None,
            key: None,
            ref_: None,
        }
    }

    #[test]
    fn test_suspense_manager_creation() {
        let manager = SuspenseManager::new();
        assert_eq!(manager.boundaries.len(), 0);
        assert_eq!(manager.promise_cache.len(), 0);
        assert_eq!(manager.boundary_stack.len(), 0);
        assert_eq!(manager.global_promise_counter, 0);
    }

    #[test]
    fn test_create_boundary() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let boundary_id = manager
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        assert_eq!(boundary_id, "test-boundary");
        assert_eq!(manager.boundaries.len(), 1);
        assert_eq!(manager.boundary_stack.len(), 1);
    }

    #[test]
    fn test_register_promise() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let boundary_id = manager
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        let promise_id = manager
            .register_promise(
                "test-component".to_string(),
                boundary_id,
                Some("cache-key-1".to_string()),
            )
            .expect("Should register promise");

        assert!(!promise_id.is_empty());
        assert_eq!(manager.promise_cache.len(), 1);

        let boundary = manager.boundaries.get("test-boundary").unwrap();
        assert_eq!(boundary.pending_promises.len(), 1);
        assert!(!boundary.resolved);
    }

    #[test]
    fn test_resolve_promise() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let boundary_id = manager
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        let promise_id = manager
            .register_promise("test-component".to_string(), boundary_id.clone(), None)
            .expect("Should register promise");

        let resolved_value = serde_json::json!({"data": "test"});
        let affected =
            manager.resolve_promise(promise_id, resolved_value).expect("Should resolve promise");

        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0], boundary_id);

        let boundary = manager.boundaries.get(&boundary_id).unwrap();
        assert!(boundary.resolved);
        assert_eq!(boundary.pending_promises.len(), 0);
    }

    #[test]
    fn test_nested_boundaries() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let parent_id = manager
            .create_boundary(
                "parent-boundary".to_string(),
                fallback.clone(),
                None,
                "/parent".to_string(),
            )
            .expect("Should create parent boundary");

        let child_id = manager
            .create_boundary(
                "child-boundary".to_string(),
                fallback,
                Some(parent_id.clone()),
                "/parent/child".to_string(),
            )
            .expect("Should create child boundary");

        let parent_boundary = manager.boundaries.get(&parent_id).unwrap();
        assert_eq!(parent_boundary.children_boundaries.len(), 1);
        assert_eq!(parent_boundary.children_boundaries[0], child_id);

        let child_boundary = manager.boundaries.get(&child_id).unwrap();
        assert_eq!(child_boundary.parent_id, Some(parent_id));
    }

    #[test]
    fn test_cleanup_resolved() {
        let mut manager = SuspenseManager::new();
        let fallback = create_test_fallback();

        let boundary_id = manager
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        let promise_id = manager
            .register_promise("test-component".to_string(), boundary_id.clone(), None)
            .expect("Should register promise");

        let resolved_value = serde_json::json!({"data": "test"});
        manager.resolve_promise(promise_id, resolved_value).expect("Should resolve promise");

        let cleaned = manager.cleanup_resolved(0);
        assert!(cleaned > 0);
    }
}
