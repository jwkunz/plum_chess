//! Threading architecture primitives for V4 parallel search.
//!
//! This module defines the concurrency configuration and shared-control types
//! used to transition the search stack to true multi-threaded execution.
//! Step 1 focuses on architecture contracts and configuration plumbing; the
//! actual parallel root/work splitting lands in later steps.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use crate::search::transposition_table_v11::{TTEntry, TTStats, TranspositionTable};

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

/// Lightweight per-thread scratch context.
///
/// These are intentionally local-only (no cross-thread shared mutable state)
/// so later parallel search can re-use existing single-thread heuristics safely.
#[derive(Debug, Clone)]
pub struct WorkerThreadContext {
    pub worker_id: usize,
    pub nodes_local: u64,
    pub split_depth_hint: u8,
}

impl WorkerThreadContext {
    #[inline]
    pub fn new(worker_id: usize) -> Self {
        Self {
            worker_id,
            nodes_local: 0,
            split_depth_hint: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.nodes_local = 0;
        self.split_depth_hint = 0;
    }
}

/// Pool of per-thread contexts owned by an engine instance.
#[derive(Debug, Clone, Default)]
pub struct ThreadContextPool {
    contexts: Vec<WorkerThreadContext>,
}

impl ThreadContextPool {
    pub fn with_threads(thread_count: usize) -> Self {
        let n = thread_count.max(1);
        let mut contexts = Vec::with_capacity(n);
        for i in 0..n {
            contexts.push(WorkerThreadContext::new(i));
        }
        Self { contexts }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }

    #[inline]
    pub fn helper_count(&self) -> usize {
        self.contexts.len().saturating_sub(1)
    }

    #[inline]
    pub fn reset(&mut self) {
        for ctx in &mut self.contexts {
            ctx.reset();
        }
    }
}

/// Shared cancellation + accounting state for future worker pools.
#[derive(Debug)]
pub struct SharedSearchState {
    stop: AtomicBool,
    pub nodes_visited: AtomicU64,
    node_budget: AtomicU64,     // 0 means unlimited
    time_budget_ms: AtomicU64,  // 0 means unlimited
    started_at: Mutex<Option<Instant>>,
}

impl SharedSearchState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            stop: AtomicBool::new(false),
            nodes_visited: AtomicU64::new(0),
            node_budget: AtomicU64::new(0),
            time_budget_ms: AtomicU64::new(0),
            started_at: Mutex::new(None),
        })
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

    #[inline]
    pub fn set_node_budget(&self, budget: Option<u64>) {
        self.node_budget.store(budget.unwrap_or(0), Ordering::Relaxed);
    }

    #[inline]
    pub fn set_time_budget_ms(&self, budget_ms: Option<u64>) {
        self.time_budget_ms
            .store(budget_ms.unwrap_or(0), Ordering::Relaxed);
    }

    #[inline]
    pub fn reset_started_at(&self) {
        if let Ok(mut guard) = self.started_at.lock() {
            *guard = Some(Instant::now());
        }
    }

    #[inline]
    pub fn reset_accounting(&self) {
        self.nodes_visited.store(0, Ordering::Relaxed);
        self.stop.store(false, Ordering::Relaxed);
        self.reset_started_at();
    }

    /// Adds node count and returns true if any configured budget is exceeded.
    #[inline]
    pub fn bump_nodes_and_check_budget(&self, n: u64) -> bool {
        let new_nodes = self.nodes_visited.fetch_add(n, Ordering::Relaxed) + n;
        let limit = self.node_budget.load(Ordering::Relaxed);
        if limit != 0 && new_nodes >= limit {
            return true;
        }
        self.time_budget_exceeded()
    }

    #[inline]
    pub fn time_budget_exceeded(&self) -> bool {
        let budget_ms = self.time_budget_ms.load(Ordering::Relaxed);
        if budget_ms == 0 {
            return false;
        }
        let Ok(guard) = self.started_at.lock() else {
            return false;
        };
        let Some(started) = *guard else {
            return false;
        };
        started.elapsed().as_millis() as u64 >= budget_ms
    }
}

/// Thread-safe transposition table façade for shared worker access.
///
/// Step 2 uses coarse-grained per-shard mutexes for simplicity and correctness.
/// A future step can replace this with lock-free or striped atomic buckets.
#[derive(Debug)]
pub struct SharedTranspositionTable {
    shards: Vec<Mutex<TranspositionTable>>,
}

impl SharedTranspositionTable {
    pub fn new_with_mb(total_mb: usize, shard_count: usize) -> Arc<Self> {
        let shards = shard_count.max(1);
        let mb_per_shard = (total_mb.max(1) / shards).max(1);
        let mut vec = Vec::with_capacity(shards);
        for _ in 0..shards {
            vec.push(Mutex::new(TranspositionTable::new_with_mb(mb_per_shard)));
        }
        Arc::new(Self { shards: vec })
    }

    #[inline]
    fn shard_idx(&self, key: u64) -> usize {
        (key as usize) % self.shards.len()
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let idx = self.shard_idx(key);
        let Ok(mut guard) = self.shards[idx].lock() else {
            return None;
        };
        guard.probe(key)
    }

    pub fn store(&self, entry: TTEntry) {
        let idx = self.shard_idx(entry.key);
        if let Ok(mut guard) = self.shards[idx].lock() {
            guard.store(entry);
        }
    }

    pub fn clear(&self) {
        for shard in &self.shards {
            if let Ok(mut guard) = shard.lock() {
                guard.clear();
            }
        }
    }

    pub fn new_generation(&self) {
        for shard in &self.shards {
            if let Ok(mut guard) = shard.lock() {
                guard.new_generation();
            }
        }
    }

    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .filter_map(|s| s.lock().ok().map(|g| g.len()))
            .sum()
    }

    pub fn stats(&self) -> TTStats {
        let mut merged = TTStats::default();
        for shard in &self.shards {
            if let Ok(guard) = shard.lock() {
                let s = guard.stats();
                merged.probes += s.probes;
                merged.hits += s.hits;
                merged.stores += s.stores;
            }
        }
        merged
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

    #[test]
    fn shared_state_budget_checks_work() {
        let state = SharedSearchState::new();
        state.reset_accounting();
        state.set_node_budget(Some(5));
        assert!(!state.bump_nodes_and_check_budget(4));
        assert!(state.bump_nodes_and_check_budget(1));
    }

    #[test]
    fn shared_tt_store_and_probe() {
        let tt = SharedTranspositionTable::new_with_mb(4, 2);
        let entry = TTEntry {
            key: 12345,
            depth: 6,
            score: 42,
            bound: crate::search::transposition_table_v11::Bound::Exact,
            best_move: Some(77),
        };
        tt.store(entry);
        let probed = tt.probe(12345).expect("entry should exist");
        assert_eq!(probed.key, 12345);
        assert_eq!(probed.score, 42);
    }

    #[test]
    fn thread_context_pool_initializes_and_resets() {
        let mut pool = ThreadContextPool::with_threads(4);
        assert_eq!(pool.len(), 4);
        assert_eq!(pool.helper_count(), 3);
        assert!(!pool.is_empty());

        pool.reset();
        // Reset should keep shape stable.
        assert_eq!(pool.len(), 4);
    }
}
