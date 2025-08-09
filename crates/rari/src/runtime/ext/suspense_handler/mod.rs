#![allow(clippy::disallowed_methods)]

use crate::rsc::serializer::ReactElement;
use crate::rsc::suspense::SuspenseManager;
use crate::runtime::JsError;
use deno_core::{Extension, OpState, extension, op2, serde_v8};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspenseHandlerConfig {
    pub max_boundary_depth: usize,
    pub cleanup_interval_ms: u64,
    pub max_promise_age_seconds: u64,
    pub enable_metrics: bool,
    pub enable_debug: bool,
}

impl Default for SuspenseHandlerConfig {
    fn default() -> Self {
        Self {
            max_boundary_depth: 50,
            cleanup_interval_ms: 30000,
            max_promise_age_seconds: 300,
            enable_metrics: true,
            enable_debug: false,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct SuspenseMetrics {
    pub total_boundaries_created: u64,
    pub total_promises_registered: u64,
    pub total_promises_resolved: u64,
    pub total_promises_rejected: u64,
    pub active_boundaries: usize,
    pub active_promises: usize,
    pub cache_hit_rate: f64,
    pub average_resolution_time_ms: f64,
    pub cleanup_runs: u64,
}

pub struct SuspenseHandler {
    manager: SuspenseManager,
    config: SuspenseHandlerConfig,
    metrics: SuspenseMetrics,
    last_cleanup: std::time::Instant,
}

impl Default for SuspenseHandler {
    fn default() -> Self {
        Self::new(SuspenseHandlerConfig::default())
    }
}

impl SuspenseHandler {
    pub fn new(config: SuspenseHandlerConfig) -> Self {
        Self {
            manager: SuspenseManager::new(),
            config,
            metrics: SuspenseMetrics {
                total_boundaries_created: 0,
                total_promises_registered: 0,
                total_promises_resolved: 0,
                total_promises_rejected: 0,
                active_boundaries: 0,
                active_promises: 0,
                cache_hit_rate: 0.0,
                average_resolution_time_ms: 0.0,
                cleanup_runs: 0,
            },
            last_cleanup: std::time::Instant::now(),
        }
    }

    pub fn create_boundary(
        &mut self,
        id: String,
        fallback: ReactElement,
        parent_id: Option<String>,
        component_path: String,
    ) -> Result<String, JsError> {
        if self.manager.get_boundary_depth() >= self.config.max_boundary_depth {
            return Err(JsError::generic("Maximum Suspense boundary depth exceeded"));
        }

        let result = self
            .manager
            .create_boundary(id, fallback, parent_id, component_path)
            .map_err(|e| JsError::generic(e.to_string()))?;

        self.metrics.total_boundaries_created += 1;
        self.metrics.active_boundaries = self.manager.boundaries_count();

        Ok(result)
    }

    pub fn register_promise(
        &mut self,
        component_id: String,
        boundary_id: String,
        cache_key: Option<String>,
    ) -> Result<String, JsError> {
        let result = self
            .manager
            .register_promise(component_id, boundary_id, cache_key)
            .map_err(|e| JsError::generic(e.to_string()))?;

        self.metrics.total_promises_registered += 1;
        self.metrics.active_promises = self.manager.promise_cache_count();

        Ok(result)
    }

    pub fn resolve_promise(
        &mut self,
        promise_id: String,
        resolved_value: serde_json::Value,
    ) -> Result<Vec<String>, JsError> {
        let result = self
            .manager
            .resolve_promise(promise_id, resolved_value)
            .map_err(|e| JsError::generic(e.to_string()))?;

        self.metrics.total_promises_resolved += 1;
        self.metrics.active_promises = self.manager.promise_cache_count();
        self.metrics.active_boundaries = self.manager.boundaries_count();

        Ok(result)
    }

    pub fn reject_promise(
        &mut self,
        promise_id: String,
        error_message: String,
    ) -> Result<Vec<String>, JsError> {
        let result = self
            .manager
            .reject_promise(promise_id, error_message)
            .map_err(|e| JsError::generic(e.to_string()))?;

        self.metrics.total_promises_rejected += 1;
        self.metrics.active_promises = self.manager.promise_cache_count();

        Ok(result)
    }

    pub fn get_boundary_state(
        &self,
        boundary_id: &str,
    ) -> Option<crate::rsc::suspense::SuspenseBoundaryState> {
        self.manager.get_boundary_state(boundary_id)
    }

    pub fn get_metrics(&self) -> &SuspenseMetrics {
        &self.metrics
    }

    pub fn cleanup_if_needed(&mut self) -> usize {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_cleanup).as_millis()
            >= self.config.cleanup_interval_ms as u128
        {
            let cleaned = self.manager.cleanup_resolved(self.config.max_promise_age_seconds);
            self.metrics.cleanup_runs += 1;
            self.metrics.active_boundaries = self.manager.boundaries_count();
            self.metrics.active_promises = self.manager.promise_cache_count();
            self.last_cleanup = now;
            cleaned
        } else {
            0
        }
    }

    pub fn get_stats(&self) -> crate::rsc::suspense::SuspenseStats {
        self.manager.get_stats()
    }

    pub fn has_pending_boundaries(&self) -> bool {
        self.manager.has_pending_boundaries()
    }

    pub fn get_current_boundary(&self) -> Option<&String> {
        self.manager.get_current_boundary()
    }

    pub fn pop_boundary(&mut self) -> Option<String> {
        self.manager.pop_boundary()
    }
}

#[op2]
#[string]
pub fn op_suspense_create_boundary(
    state: &mut OpState,
    #[string] id: String,
    #[serde] _fallback: serde_v8::Value,
    #[string] parent_id: Option<String>,
    #[string] component_path: String,
) -> Result<String, JsError> {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    let fallback_element = ReactElement {
        element_type: crate::rsc::serializer::ElementType::Text("Loading...".to_string()),
        props: None,
        key: None,
        ref_: None,
    };

    handler.create_boundary(id, fallback_element, parent_id, component_path)
}

#[op2]
#[string]
pub fn op_suspense_register_promise(
    state: &mut OpState,
    #[string] component_id: String,
    #[string] boundary_id: String,
    #[string] cache_key: Option<String>,
) -> Result<String, JsError> {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    handler.register_promise(component_id, boundary_id, cache_key)
}

#[op2]
#[serde]
pub fn op_suspense_resolve_promise(
    state: &mut OpState,
    #[string] promise_id: String,
    #[serde] resolved_value: serde_json::Value,
) -> Result<Vec<String>, JsError> {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    handler.resolve_promise(promise_id, resolved_value)
}

#[op2]
#[serde]
pub fn op_suspense_reject_promise(
    state: &mut OpState,
    #[string] promise_id: String,
    #[string] error_message: String,
) -> Result<Vec<String>, JsError> {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    handler.reject_promise(promise_id, error_message)
}

#[op2]
#[serde]
pub fn op_suspense_get_boundary_state(
    state: &mut OpState,
    #[string] boundary_id: String,
) -> Option<serde_json::Value> {
    let suspense_handler = state.borrow::<Rc<RefCell<SuspenseHandler>>>();
    let handler = suspense_handler.borrow();

    handler.get_boundary_state(&boundary_id).map(|state| {
        serde_json::json!({
            "resolved": state.resolved,
            "pending_count": state.pending_count,
            "has_error": state.error.is_some(),
            "error": state.error.as_ref().map(|e| &e.message)
        })
    })
}

#[op2]
#[serde]
pub fn op_suspense_get_metrics(state: &mut OpState) -> Result<SuspenseMetrics, JsError> {
    let suspense_handler = state.borrow::<Rc<RefCell<SuspenseHandler>>>();
    let handler = suspense_handler.borrow();

    Ok(handler.get_metrics().clone())
}

#[op2(fast)]
pub fn op_suspense_cleanup(state: &mut OpState) -> u32 {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    handler.cleanup_if_needed() as u32
}

#[op2]
#[serde]
pub fn op_suspense_get_stats(state: &mut OpState) -> Result<serde_json::Value, JsError> {
    let suspense_handler = state.borrow::<Rc<RefCell<SuspenseHandler>>>();
    let handler = suspense_handler.borrow();

    let stats = handler.get_stats();
    Ok(serde_json::json!({
        "total_boundaries": stats.total_boundaries,
        "resolved_boundaries": stats.resolved_boundaries,
        "pending_boundaries": stats.pending_boundaries,
        "error_boundaries": stats.error_boundaries,
        "total_promises": stats.total_promises,
        "resolved_promises": stats.resolved_promises,
        "pending_promises": stats.pending_promises,
        "rejected_promises": stats.rejected_promises,
        "cached_promises": stats.cached_promises,
        "boundary_stack_depth": stats.boundary_stack_depth
    }))
}

#[op2]
#[string]
pub fn op_suspense_get_current_boundary(state: &mut OpState) -> Option<String> {
    let suspense_handler = state.borrow::<Rc<RefCell<SuspenseHandler>>>();
    let handler = suspense_handler.borrow();

    handler.get_current_boundary().cloned()
}

#[op2]
#[string]
pub fn op_suspense_pop_boundary(state: &mut OpState) -> Option<String> {
    let suspense_handler = state.borrow_mut::<Rc<RefCell<SuspenseHandler>>>();
    let mut handler = suspense_handler.borrow_mut();

    handler.pop_boundary()
}

#[op2(fast)]
pub fn op_suspense_has_pending(state: &mut OpState) -> bool {
    let suspense_handler = state.borrow::<Rc<RefCell<SuspenseHandler>>>();
    let handler = suspense_handler.borrow();

    handler.has_pending_boundaries()
}

extension!(
    rari_suspense_handler,
    ops = [
        op_suspense_create_boundary,
        op_suspense_register_promise,
        op_suspense_resolve_promise,
        op_suspense_reject_promise,
        op_suspense_get_boundary_state,
        op_suspense_get_metrics,
        op_suspense_cleanup,
        op_suspense_get_stats,
        op_suspense_get_current_boundary,
        op_suspense_pop_boundary,
        op_suspense_has_pending,
    ],
    options = {
        config: SuspenseHandlerConfig,
    },
    state = |state, options| {
        state.put(Rc::new(RefCell::new(SuspenseHandler::new(options.config))));
    },
);

pub fn extensions(_is_snapshot: bool) -> Vec<Extension> {
    vec![rari_suspense_handler::init(SuspenseHandlerConfig::default())]
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
    fn test_suspense_handler_creation() {
        let config = SuspenseHandlerConfig::default();
        let handler = SuspenseHandler::new(config);

        assert_eq!(handler.metrics.total_boundaries_created, 0);
        assert_eq!(handler.metrics.total_promises_registered, 0);
        assert_eq!(handler.metrics.active_boundaries, 0);
    }

    #[test]
    fn test_create_boundary() {
        let mut handler = SuspenseHandler::default();
        let fallback = create_test_fallback();

        let boundary_id = handler
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        assert_eq!(boundary_id, "test-boundary");
        assert_eq!(handler.metrics.total_boundaries_created, 1);
        assert_eq!(handler.metrics.active_boundaries, 1);
    }

    #[test]
    fn test_register_and_resolve_promise() {
        let mut handler = SuspenseHandler::default();
        let fallback = create_test_fallback();

        let boundary_id = handler
            .create_boundary(
                "test-boundary".to_string(),
                fallback,
                None,
                "/test/component".to_string(),
            )
            .expect("Should create boundary");

        let promise_id = handler
            .register_promise(
                "test-component".to_string(),
                boundary_id,
                Some("cache-key-1".to_string()),
            )
            .expect("Should register promise");

        assert!(!promise_id.is_empty());
        assert_eq!(handler.metrics.total_promises_registered, 1);

        let resolved_value = serde_json::json!({"data": "test"});
        let affected =
            handler.resolve_promise(promise_id, resolved_value).expect("Should resolve promise");

        assert!(!affected.is_empty());
        assert_eq!(handler.metrics.total_promises_resolved, 1);
    }

    #[test]
    fn test_boundary_depth_limit() {
        let config = SuspenseHandlerConfig { max_boundary_depth: 2, ..Default::default() };
        let mut handler = SuspenseHandler::new(config);
        let fallback = create_test_fallback();

        handler
            .create_boundary(
                "boundary-1".to_string(),
                fallback.clone(),
                None,
                "/test/1".to_string(),
            )
            .expect("Should create first boundary");

        handler
            .create_boundary(
                "boundary-2".to_string(),
                fallback.clone(),
                Some("boundary-1".to_string()),
                "/test/2".to_string(),
            )
            .expect("Should create second boundary");

        let result = handler.create_boundary(
            "boundary-3".to_string(),
            fallback,
            Some("boundary-2".to_string()),
            "/test/3".to_string(),
        );

        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("depth exceeded"));
    }
}
