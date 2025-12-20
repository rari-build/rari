use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::SuspenseBoundaryInfo;

pub struct SuspenseBoundaryManager {
    boundaries: Arc<Mutex<FxHashMap<String, SuspenseBoundaryInfo>>>,
    boundary_stack: Vec<String>,
    resolved_boundaries: Arc<Mutex<FxHashMap<String, serde_json::Value>>>,
    rendered_skeleton_ids: Arc<Mutex<FxHashSet<String>>>,
}

impl Default for SuspenseBoundaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspenseBoundaryManager {
    pub fn new() -> Self {
        Self {
            boundaries: Arc::new(Mutex::new(FxHashMap::default())),
            boundary_stack: Vec::new(),
            resolved_boundaries: Arc::new(Mutex::new(FxHashMap::default())),
            rendered_skeleton_ids: Arc::new(Mutex::new(FxHashSet::default())),
        }
    }

    pub async fn register_boundary(&mut self, mut boundary: SuspenseBoundaryInfo) {
        let boundary_id = boundary.id.clone();

        boundary.skeleton_rendered = false;
        boundary.is_resolved = false;

        {
            let mut boundaries = self.boundaries.lock().await;
            boundaries.insert(boundary_id.clone(), boundary);
        }
        self.boundary_stack.push(boundary_id);
    }

    pub async fn mark_skeleton_rendered(&self, boundary_id: &str) -> bool {
        let mut skeleton_ids = self.rendered_skeleton_ids.lock().await;
        let is_first = skeleton_ids.insert(boundary_id.to_string());

        {
            let mut boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get_mut(boundary_id) {
                boundary.skeleton_rendered = true;
            }
        }

        is_first
    }

    pub async fn resolve_boundary(&self, boundary_id: &str, content: serde_json::Value) {
        {
            let mut resolved = self.resolved_boundaries.lock().await;
            resolved.insert(boundary_id.to_string(), content);
        }

        {
            let mut boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get_mut(boundary_id) {
                boundary.pending_promise_count = 0;
                boundary.is_resolved = true;
            }
        }

        {
            let mut skeleton_ids = self.rendered_skeleton_ids.lock().await;
            skeleton_ids.remove(boundary_id);
        }
    }

    pub async fn get_pending_boundaries(&self) -> Vec<SuspenseBoundaryInfo> {
        let boundaries = self.boundaries.lock().await;
        let resolved = self.resolved_boundaries.lock().await;

        boundaries
            .values()
            .filter(|b| !resolved.contains_key(&b.id) && b.pending_promise_count > 0)
            .cloned()
            .collect()
    }

    pub async fn validate_no_duplicate_skeletons(&self) -> Vec<String> {
        let boundaries = self.boundaries.lock().await;
        let skeleton_ids = self.rendered_skeleton_ids.lock().await;

        let mut duplicates = Vec::new();

        for (id, boundary) in boundaries.iter() {
            if boundary.skeleton_rendered && !skeleton_ids.contains(id) && !boundary.is_resolved {
                duplicates.push(id.clone());
            }
        }

        if !duplicates.is_empty() {
            tracing::error!(
                "Duplicate skeleton validation failed: {} boundaries have inconsistent state",
                duplicates.len()
            );
        }

        duplicates
    }

    pub async fn get_rendered_skeleton_count(&self) -> usize {
        self.rendered_skeleton_ids.lock().await.len()
    }

    pub async fn has_rendered_skeleton(&self, boundary_id: &str) -> bool {
        self.rendered_skeleton_ids.lock().await.contains(boundary_id)
    }
}
