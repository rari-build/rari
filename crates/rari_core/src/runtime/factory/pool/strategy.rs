use std::sync::atomic::{AtomicUsize, Ordering};

pub trait PickStrategy: Send + Sync {
    fn pick(&self, healthy_indices: &[usize], next: &AtomicUsize) -> Option<usize>;
}

#[non_exhaustive]
pub struct RoundRobinStrategy;

impl PickStrategy for RoundRobinStrategy {
    fn pick(&self, healthy_indices: &[usize], next: &AtomicUsize) -> Option<usize> {
        if healthy_indices.is_empty() {
            None
        } else {
            let pos = next.fetch_add(1, Ordering::Relaxed) % healthy_indices.len();
            Some(healthy_indices[pos])
        }
    }
}
