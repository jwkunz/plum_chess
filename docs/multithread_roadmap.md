# Multithread Roadmap (Major Version 4)

This document captures the implementation roadmap for transforming Plum Chess
into a true multi-threaded engine, and marks each step as completed.

## Completed Steps

1. `v4.1` Threading architecture scaffolding
- Added thread model/config abstractions and shared search state contracts.
- Introduced thread-safe TT façade primitives.

2. `v4.2` Thread-safe budgets and cancellation primitives
- Added node/time budget accounting.
- Added shared stop propagation utilities.

3. `v4.3` Per-thread context pool
- Added reusable worker context containers.
- Connected context lifecycle to engine instance lifecycle.

4. `v4.4` Initial parallel root split (Lazy SMP phase 1)
- Implemented top-level move split across worker threads.
- Added merge, panic handling, and serial fallback behavior.

5. `v4.5` Parallel root integrated into main move selection path
- Enabled threaded root ranking for normal best-move flow.
- Kept deterministic fallback and compatibility with existing UCI output.

6. `v4.6` Shared budget-aware worker cancellation
- Workers now stop collectively on shared node/time budget exhaustion.
- Added explicit budget-stop telemetry markers.

7. `v4.7` Deterministic threaded mode
- Added deterministic mode to disable non-deterministic parallel behavior.
- Exposed mode through UCI options.

8. `v4.8` Hardening and telemetry
- Added split telemetry and panic fallback instrumentation.
- Improved resilience of asynchronous search plumbing.

9. `v4.9` Shared TT refinement for threaded root search
- Added shared TT probing/storing in root worker path.
- Exposed shared TT hit/probe/store diagnostics.

10. `v4.10` Dynamic root work balancing
- Replaced static stride splitting with atomic work queue assignment.
- Improved load balancing when root branches have uneven cost.

11. `v4.11` TT sharding and worker-overhead tuning
- Made shared TT sharding proportional to thread count.
- Reduced worker-local TT memory pressure.

12. `v4.12` Adaptive worker scaling by budget
- Added budget-aware worker count reduction for tiny node/time allocations.
- Avoids over-threading overhead in short searches.

13. `v4.13` UCI controls for parallel thresholds
- Added `RootParallelMinDepth` and `RootParallelMinMoves`.
- Wired options into sync/async UCI engine setup.

14. `v4.14` Thread scaling benchmark harness
- Added standalone benchmark binary:
  - `src/bin/thread_scaling_bench.rs`
- Supports empirical scaling tests by thread count and depth.

## Outcome

The major-version threading roadmap is complete through root-level parallel
search, with shared cancellation, shared TT support, UCI controls, and
benchmarking support.

## Recommended Ongoing Loop

1. Run thread-scaling benchmark:
- `cargo run --bin thread_scaling_bench -- 8 4 3`

2. Compare `Threads`, `RootParallelMinDepth`, and `RootParallelMinMoves`
settings in your GUI.

3. Tune by time-control class:
- Blitz: higher min-depth/min-moves to avoid thread overhead.
- Rapid/classical: lower thresholds to exploit parallelism earlier.
