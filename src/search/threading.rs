//! Threading architecture primitives for V4 parallel search.
//!
//! This module defines the concurrency configuration and shared-control types
//! used to transition the search stack to true multi-threaded execution.
//! Step 1 focuses on architecture contracts and configuration plumbing; the
//! actual parallel root/work splitting lands in later steps.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

/// Search execution model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadingModel {
    /// Classic single-threaded search path.
    SingleThreaded,
    /// Lazy SMP model with helper workers sharing TT and stop conditions.
    LazySmp,
}

/// Threading configuration owned by the engine instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadingConfig {
    pub model: ThreadingModel,
    pub requested_threads: usize,
}

impl Default for ThreadingConfig {
    fn default() -> Self {
        Self {
            model: ThreadingModel::LazySmp,
            requested_threads: 1,
        }
    }
}

impl ThreadingConfig {
    #[inline]
    pub fn normalized_threads(self) -> usize {
        self.requested_threads.max(1)
    }

    #[inline]
    pub fn helper_threads(self) -> usize {
        self.normalized_threads().saturating_sub(1)
    }
}

/// Shared cancellation + accounting state for future worker pools.
#[derive(Debug, Default)]
pub struct SharedSearchState {
    stop: AtomicBool,
    pub nodes_visited: AtomicU64,
}

impl SharedSearchState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    #[inline]
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    #[inline]
    pub fn should_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn add_nodes(&self, n: u64) {
        self.nodes_visited.fetch_add(n, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threading_config_normalizes_threads() {
        let cfg = ThreadingConfig {
            model: ThreadingModel::LazySmp,
            requested_threads: 0,
        };
        assert_eq!(cfg.normalized_threads(), 1);
        assert_eq!(cfg.helper_threads(), 0);
    }

    #[test]
    fn shared_state_stop_and_node_accounting() {
        let state = SharedSearchState::new();
        assert!(!state.should_stop());
        state.request_stop();
        assert!(state.should_stop());

        state.add_nodes(10);
        assert_eq!(state.nodes_visited.load(Ordering::Relaxed), 10);
    }
}
