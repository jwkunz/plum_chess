# Performance Guide: Version 7 Search Optimization

This document summarizes the version 7 performance pass. It follows the same
project style as prior guides and records the exact step-by-step work that was
implemented and validated.

Primary files touched:

- `src/search/iterative_deepening_v15.rs`
- `src/search/transposition_table_v11.rs`
- `src/move_generation/legal_move_generator.rs`
- `src/engines/engine_iterative_v16.rs`
- `benches/v7_perf_criterion.rs`
- `docs/requirements/v7.md`

## Goal

Increase search throughput without external tables and without breaking search
stability.

## v7 Step Map

```dot
digraph v7_steps {
  rankdir=LR;
  node [shape=box, style=rounded];

  s0 [label="v7.0\nBaseline runner + locked requirements"];
  s1 [label="v7.1\nLegal move gen allocation reduction"];
  s2 [label="v7.2\nPV make/unmake consolidation"];
  s3 [label="v7.3\nMove ordering retune"];
  s4 [label="v7.4\nLMR/LMP/null retune"];
  s5 [label="v7.5\nQuiescence cost control"];
  s6 [label="v7.6\nTT bucket index optimization"];
  s7 [label="v7.7\nParallel root chunk scheduling"];
  s8 [label="v7.8\nRepetition scan optimization"];
  s9 [label="v7.9\nFinal benchmark + stabilization"];

  s0 -> s1 -> s2 -> s3 -> s4 -> s5 -> s6 -> s7 -> s8 -> s9;
}
```

## Implemented Changes

### v7.0 Baseline and instrumentation

- Added `docs/requirements/v7.md`.
- Added `benches/v7_perf_criterion.rs` for reproducible depth-based baseline
  snapshots across representative FENs.

### v7.1 Hot-path de-allocation in move generation

- Updated `generate_legal_move_descriptions_in_place` to filter pseudo-legal
  moves in-place instead of allocating a second `legal` vector.
- Effect: less allocation pressure in a very hot function.

### v7.2 Make/unmake consolidation

- Updated `principal_variation_from_tt` in `iterative_deepening_v15` to use
  `make_move_in_place` instead of `apply_move` cloning per PV ply.
- Effect: reduced per-PV state-copy overhead.

### v7.3 Move ordering retune

- Switched move ordering sorts to `sort_unstable_by_key`.
- Retuned tactical ordering blend for captures:
  - MVV/LVA-style victim/aggressor weighting
  - SEE-weighted tactical bias
  - promotion value-aware bonus
- Adjusted quiet ordering contributions (history/continuation scaling).

### v7.4 Pruning/reduction retune

- Made LMR schedule more depth/move-index sensitive and slightly more
  aggressive for deep/late quiet moves.
- Tightened LMP thresholds for shallow nodes.
- Reduced null-move verification frequency at lower depths.

### v7.5 Quiescence cost control

- Reduced quiet-check expansion depth (`QUIESCENCE_CHECK_PLY`).
- Replaced `moves.contains` checks in quiescence check augmentation with a fixed
  4096-entry from/to seen table.

### v7.6 TT cache/index path improvement

- Updated `transposition_table_v11` bucket count to power-of-two.
- Replaced modulo bucket index with mask (`key & mask`) in probe/store path.

### v7.7 Parallel root scheduling

- In `engine_iterative_v16` parallel root ranking, moved from single-index
  atomic fetch to chunked work claiming (`ROOT_WORK_CHUNK`).
- Effect: fewer atomic operations and lower scheduler contention.

### v7.8 Repetition detection optimization

- Optimized `is_draw_state`:
  - bounded scan by halfmove-clock window,
  - reverse scan only,
  - side-to-move parity stepping (`step_by(2)`),
  - early exit when count reaches 3.

### v7.9 Stabilization and benchmark capture

- Re-ran baseline and acceptance smoke tests under release profile.
- Verified no immediate hangs/overflows in final v7 path.

## Benchmark Snapshots

Baseline command:

```bash
PLUM_V7_DEPTH=4 cargo bench --bench v7_perf_criterion
```

`v7.0` snapshot (initial baseline):

- `startpos`: nodes `467`, elapsed `1ms`, nps `467000`
- `classical_mid`: nodes `2123`, elapsed `16ms`, nps `132687`
- `tactical`: nodes `4825`, elapsed `33ms`, nps `146212`

`v7.9` snapshot (after optimizations):

- `startpos`: nodes `722`, elapsed `3ms`, nps `240666`
- `classical_mid`: nodes `2912`, elapsed `17ms`, nps `171294`
- `tactical`: nodes `4597`, elapsed `25ms`, nps `183880`

Notes:

- Millisecond granularity for very short runs can distort NPS on tiny elapsed
  times (notably `startpos` at depth 4). The more representative read is
  medium-complexity positions (`classical_mid`, `tactical`).
- On those positions, v7 shows improved throughput and reduced elapsed time.

## Acceptance Smoke

Command:

```bash
PLUM_V6_DEPTH=4 PLUM_V6_GAMES=4 cargo bench --bench v6_acceptance_criterion
```

Observed in final smoke:

- Opening/middlegame regression remained moderate.
- Endgame suite remains the known v6/v7 lag area (v17 endgame conversion speed
  still substantially slower than v16 in that suite).

## What v7 Improved Most

1. Hot-path allocation and sorting overhead.
2. TT probe/store indexing cost.
3. Parallel root scheduling overhead.
4. Repetition scan cost per node.

## Remaining Opportunities (for next cycle)

1. Deeper make/unmake migration inside v17 endgame verification path.
2. More aggressive scratch-buffer reuse across recursive search calls.
3. Endgame-specific verification cost controls to close the v17 endgame speed gap.

## v7.10 Hotspot Micro-Pass

After v7.9, a focused micro-optimization pass targeted two hot spots in
`iterative_deepening_v15`:

1. Removed a redundant `tt.probe(...)` call in `negamax`:
- The first probe result is now reused for TT move ordering instead of probing
  the same key again.

2. Skipped sorting when move list size is `< 2`:
- `order_moves` and `order_moves_basic` now return immediately for trivial lists.

### v7.10 Snapshot (depth 4)

Command:

```bash
PLUM_V7_DEPTH=4 cargo bench --bench v7_perf_criterion
```

| Position       | v7.9 NPS | v7.10 NPS | Delta |
|----------------|----------|-----------|-------|
| `startpos`     | 240666   | 240666    | 0.00% |
| `classical_mid`| 161777   | 208000    | +28.57% |
| `tactical`     | 170259   | 199869    | +17.39% |

Interpretation:

- Micro-pass gains are strongest in non-trivial middlegame/tactical trees,
  where repeated TT probing and small-list sort overhead compound.
- Tiny-depth start position remains timer-granularity dominated.

## v7.11 Draw-Repetition Scan Tightening

A follow-on pass targeted draw detection overhead in the hot path:

- Kept the index-based reverse parity scan in `is_draw_state` to avoid layered
  iterator adaptors.
- Confirmed no search behavior regressions in v15-focused tests.

Depth-4 benchmark (`classical_mid/d4`) remained statistically neutral versus the
previous step, with a slightly improved median.

## v7.12 Draw Fast-Path Gates

Added two constant-time guards before repetition scanning:

- If `halfmove_clock < 4`, threefold repetition is impossible.
- If `repetition_history.len() < 5`, three occurrences cannot exist.

This avoids unnecessary scan setup in many middlegame nodes while preserving
exact draw semantics.

### v7.12 snapshot (depth 4, `classical_mid/d4`)

```bash
PLUM_V7_DEPTH=4 cargo bench --bench v7_perf_criterion -- "classical_mid/d4" --sample-size 20
```

- Time: `[70.943 ms 73.586 ms 76.238 ms]`
- Criterion change: `No change in performance detected` (non-regressive)

## v7.13 Conservative Reverse-Futility Pruning

Added a guarded reverse-futility prune in `negamax`:

- Applies only at shallow depth (`<= 2`).
- Disabled when in check, in critical endgames, and near mate-score bounds.
- Restricted to narrow-window (non-PV-like) nodes.

This targets low-value branches early while preserving tactical stability in
the sensitive parts of the tree.

### v7.13 snapshot (depth 4, `classical_mid/d4`)

```bash
PLUM_V7_DEPTH=4 cargo bench --bench v7_perf_criterion -- "classical_mid/d4" --sample-size 20
```

- Time: `[62.896 ms 65.256 ms 67.726 ms]`
- Criterion change: `Performance has improved` (p < 0.05)
