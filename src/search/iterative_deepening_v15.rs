//! Iterative deepening search with negamax alpha-beta pruning (V15).
//!
//! Implements depth-progressive search that repeatedly refines best-move
//! output and supports configurable search depth limits.
//!
//! V15 heuristics:
//! - Contempt + draw-avoidance when clearly winning.
//! - Late-endgame check extension.
//! - Killer/history move ordering.
//! - Late Move Reductions (LMR) with re-search on fail-high.
//! - Aspiration windows around previous-iteration score.
//! - Null-move pruning with basic zugzwang safeguards.
//! - Principal Variation Search (PVS) for non-PV move zero-window probing.
//! - Countermove and continuation-history move ordering.
//! - SEE-style tactical pruning and ordering in quiescence/captures.
//! - Transposition-table generation aging (depth+age replacement policy).
//! - Late Move Pruning (LMP) for low-depth late quiet moves.
//! - Null-move verification search to reduce tactical over-pruning.
//! - 4-way bucketed TT with depth/bound/age replacement policy.
//! - Deeper quiescence with selective quiet-check expansion.
//! - Stronger SEE thresholds for tactical pruning/order quality.
//! - Mate-distance consistency audit for TT store/probe normalization.
//! - Mate-score shaping via fail-soft cutoff propagation.
//! - Selective endgame extensions (checking, advanced passers, king-pawn races).

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::{make_move_in_place, unmake_move_in_place};
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::move_generation::move_generator::{MoveGenResult, MoveGenerationError, MoveGenerator};
use crate::moves::move_descriptions::{
    move_captured_piece_code, move_moved_piece_code, move_promotion_piece_code, move_to,
    piece_kind_from_code, FLAG_CAPTURE, FLAG_EN_PASSANT, NO_PIECE_CODE,
};
use crate::search::board_scoring::BoardScorer;
use crate::search::transposition_table_v11::{Bound, TTEntry, TTStats, TranspositionTable};
use crate::utils::long_algebraic::move_description_to_long_algebraic;
use std::sync::{atomic::Ordering, Arc};
use std::time::{Duration, Instant};

const MATE_SCORE: i32 = 30000;
const MAX_PLY: usize = 128;
const QUIESCENCE_DELTA_MARGIN: i32 = 120;
const SEE_BAD_CAPTURE_THRESHOLD: i32 = -120;
const QUIESCENCE_MAX_PLY: u8 = 10;
const QUIESCENCE_CHECK_PLY: u8 = 1;
const MATE_TT_THRESHOLD: i32 = MATE_SCORE - 1000;

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub max_depth: u8,
    pub movetime_ms: Option<u64>,
    pub max_nodes: Option<u64>,
    pub stop_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: 4,
            movetime_ms: None,
            max_nodes: None,
            stop_flag: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchResult {
    pub best_move: Option<u64>,
    pub best_score: i32,
    pub reached_depth: u8,
    pub nodes: u64,
    pub elapsed_ms: u64,
    pub nps: u64,
    pub tt_stats: TTStats,
}

#[derive(Debug, Clone, Default)]
pub struct PrincipalVariation {
    pub moves: Vec<u64>,
}

pub fn iterative_deepening_search<G: MoveGenerator, S: BoardScorer>(
    game_state: &GameState,
    generator: &G,
    scorer: &S,
    config: SearchConfig,
) -> MoveGenResult<SearchResult> {
    let mut local_tt = TranspositionTable::new_with_mb(16);
    iterative_deepening_search_with_tt(game_state, generator, scorer, config, &mut local_tt)
}

pub fn iterative_deepening_search_with_tt<G: MoveGenerator, S: BoardScorer>(
    game_state: &GameState,
    _generator: &G,
    scorer: &S,
    config: SearchConfig,
    tt: &mut TranspositionTable,
) -> MoveGenResult<SearchResult> {
    let started_at = Instant::now();
    let mut heuristics = SearchHeuristics::default();
    let stop_flag = config.stop_flag.as_ref();
    let max_nodes = config.max_nodes.filter(|n| *n > 0);
    let deadline = config
        .movetime_ms
        .map(|ms| started_at + Duration::from_millis(ms.max(1)));

    if config.max_depth == 0 {
        let elapsed_ms = started_at.elapsed().as_millis() as u64;
        return Ok(SearchResult {
            best_move: None,
            best_score: scorer.score(game_state),
            reached_depth: 0,
            nodes: 1,
            elapsed_ms,
            nps: 0,
            tt_stats: tt.stats(),
        });
    }

    let mut result = SearchResult::default();
    let mut total_nodes = 0u64;

    let mut prev_iter_score = 0i32;
    for depth in 1..=config.max_depth {
        if should_abort(deadline, stop_flag, total_nodes, max_nodes) {
            break;
        }
        let node_cap = max_nodes.map(|cap| cap.saturating_sub(total_nodes));
        if node_cap == Some(0) {
            break;
        }

        tt.new_generation();
        let mut nodes = 0u64;
        let mut root_state = game_state.clone();
        heuristics.reset_iteration();
        let Some((best_move, best_score)) = search_root_with_aspiration(
            &mut root_state,
            scorer,
            depth,
            prev_iter_score,
            &mut nodes,
            deadline,
            node_cap,
            stop_flag,
            tt,
            &mut heuristics,
        )?
        else {
            break;
        };

        total_nodes = total_nodes.saturating_add(nodes);
        result.best_move = best_move;
        result.best_score = best_score;
        result.reached_depth = depth;
        result.nodes = total_nodes;
        prev_iter_score = best_score;
    }

    result.elapsed_ms = started_at.elapsed().as_millis() as u64;
    result.nps = if result.elapsed_ms == 0 {
        0
    } else {
        result.nodes.saturating_mul(1000) / result.elapsed_ms
    };
    result.tt_stats = tt.stats();

    Ok(result)
}

#[inline]
fn should_abort(
    deadline: Option<Instant>,
    stop_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
    nodes: u64,
    max_nodes: Option<u64>,
) -> bool {
    if let Some(cap) = max_nodes {
        if nodes >= cap {
            return true;
        }
    }
    if let Some(limit) = deadline {
        if Instant::now() >= limit {
            return true;
        }
    }
    if let Some(flag) = stop_flag {
        if flag.load(Ordering::Relaxed) {
            return true;
        }
    }
    false
}

fn negamax_root<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    nodes: &mut u64,
    deadline: Option<Instant>,
    node_cap: Option<u64>,
    stop_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
    tt: &mut TranspositionTable,
    heuristics: &mut SearchHeuristics,
) -> MoveGenResult<Option<(Option<u64>, i32)>> {
    let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
    if moves.is_empty() {
        let score = terminal_score(game_state, 0);
        *nodes += 1;
        return Ok(Some((None, score)));
    }

    let tt_move = tt.probe(game_state.zobrist_key).and_then(|e| e.best_move);
    order_moves(
        &mut moves,
        tt_move,
        None,
        heuristics.killers_at(0),
        heuristics,
        game_state.side_to_move,
    );

    let mut best_move = None;
    let mut best_score = -MATE_SCORE;

    for mv in moves {
        if should_abort(deadline, stop_flag, *nodes, node_cap) {
            return Ok(None);
        }

        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;

        let score_opt = negamax(
            game_state,
            scorer,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            1,
            true,
            true,
            Some(mv),
            nodes,
            deadline,
            node_cap,
            stop_flag,
            tt,
            heuristics,
        )?;

        unmake_move_in_place(game_state).map_err(|x| {
            MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
        })?;

        let Some(score) = score_opt else {
            return Ok(None);
        };
        let score = -score;

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            break;
        }
    }

    Ok(Some((best_move, best_score)))
}

fn search_root_with_aspiration<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    depth: u8,
    prev_score: i32,
    nodes: &mut u64,
    deadline: Option<Instant>,
    node_cap: Option<u64>,
    stop_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
    tt: &mut TranspositionTable,
    heuristics: &mut SearchHeuristics,
) -> MoveGenResult<Option<(Option<u64>, i32)>> {
    if depth <= 1 {
        return negamax_root(
            game_state,
            scorer,
            depth,
            -MATE_SCORE,
            MATE_SCORE,
            nodes,
            deadline,
            node_cap,
            stop_flag,
            tt,
            heuristics,
        );
    }

    let mut window = aspiration_initial_window(depth);
    let mut attempts = 0u8;
    let mut alpha = (prev_score - window).max(-MATE_SCORE);
    let mut beta = (prev_score + window).min(MATE_SCORE);

    loop {
        attempts = attempts.saturating_add(1);
        let Some((best_move, score)) = negamax_root(
            game_state, scorer, depth, alpha, beta, nodes, deadline, node_cap, stop_flag, tt,
            heuristics,
        )?
        else {
            return Ok(None);
        };

        // If we've expanded to the full legal score window, accept the result.
        // This avoids pathological loops when mate scores sit on the bounds.
        if alpha <= -MATE_SCORE && beta >= MATE_SCORE {
            return Ok(Some((best_move, score)));
        }

        if score <= alpha {
            window = (window * 2).min(MATE_SCORE / 2);
            alpha = (score - window).saturating_sub(1).max(-MATE_SCORE);
            beta = (score + window).min(MATE_SCORE);
            if attempts >= 8 {
                alpha = -MATE_SCORE;
                beta = MATE_SCORE;
            }
            continue;
        }

        if score >= beta {
            window = (window * 2).min(MATE_SCORE / 2);
            alpha = (score - window).max(-MATE_SCORE);
            beta = (score + window).saturating_add(1).min(MATE_SCORE);
            if attempts >= 8 {
                alpha = -MATE_SCORE;
                beta = MATE_SCORE;
            }
            continue;
        }

        return Ok(Some((best_move, score)));
    }
}

#[inline]
fn aspiration_initial_window(depth: u8) -> i32 {
    25 + (i32::from(depth) * 10)
}

fn negamax<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    allow_check_extension: bool,
    allow_null_pruning: bool,
    prev_move: Option<u64>,
    nodes: &mut u64,
    deadline: Option<Instant>,
    node_cap: Option<u64>,
    stop_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
    tt: &mut TranspositionTable,
    heuristics: &mut SearchHeuristics,
) -> MoveGenResult<Option<i32>> {
    if should_abort(deadline, stop_flag, *nodes, node_cap) {
        return Ok(None);
    }

    if is_draw_state(game_state) {
        return Ok(Some(repetition_draw_score(scorer.score(game_state))));
    }

    let alpha_orig = alpha;

    if let Some(entry) = tt.probe(game_state.zobrist_key) {
        let tt_score = tt_score_from_storage(entry.score, ply);
        if entry.depth >= depth {
            match entry.bound {
                Bound::Exact => return Ok(Some(tt_score)),
                Bound::Lower if tt_score >= beta => return Ok(Some(tt_score)),
                Bound::Upper if tt_score <= alpha => return Ok(Some(tt_score)),
                _ => {}
            }
        }
    }

    *nodes += 1;

    if depth == 0 {
        return quiescence(
            game_state, scorer, alpha, beta, 0, nodes, deadline, node_cap, stop_flag,
        );
    }

    let in_check = is_king_in_check(game_state, game_state.side_to_move);
    if allow_null_pruning && should_try_null_move(depth, in_check, beta, game_state) {
        let null = make_null_move(game_state);
        let reduction = if depth >= 6 { 3 } else { 2 };
        let score_opt = negamax(
            game_state,
            scorer,
            depth.saturating_sub(1 + reduction),
            -beta,
            -beta + 1,
            ply.saturating_add(1),
            false,
            false,
            None,
            nodes,
            deadline,
            node_cap,
            stop_flag,
            tt,
            heuristics,
        )?;
        unmake_null_move(game_state, null);

        let Some(score) = score_opt else {
            return Ok(None);
        };
        let score = -score;
        if score >= beta {
            if should_verify_null_cutoff(depth, in_check) {
                let verify_opt = negamax(
                    game_state,
                    scorer,
                    depth.saturating_sub(1),
                    beta.saturating_sub(1),
                    beta,
                    ply,
                    allow_check_extension,
                    false,
                    prev_move,
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?;
                let Some(verify_score) = verify_opt else {
                    return Ok(None);
                };
                if verify_score >= beta {
                    return Ok(Some(verify_score));
                }
            } else {
                return Ok(Some(score));
            }
        }
    }

    let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
    if moves.is_empty() {
        return Ok(Some(terminal_score(game_state, ply)));
    }

    let tt_move = tt.probe(game_state.zobrist_key).and_then(|entry| {
        if entry.depth >= depth {
            entry.best_move
        } else {
            None
        }
    });
    let ply_idx = usize::from(ply).min(MAX_PLY - 1);
    order_moves(
        &mut moves,
        tt_move,
        prev_move,
        heuristics.killers_at(ply_idx),
        heuristics,
        game_state.side_to_move,
    );

    let mut best = -MATE_SCORE;
    let mut best_move: Option<u64> = None;

    for (move_index, mv) in moves.into_iter().enumerate() {
        if should_abort(deadline, stop_flag, *nodes, node_cap) {
            return Ok(None);
        }

        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;

        let child = child_depth(depth, game_state, allow_check_extension, mv);
        let child_allow_check_ext =
            child_allows_check_extension(depth, game_state, allow_check_extension, mv);
        let is_quiet = is_quiet_move(mv);
        if should_lmp_prune(
            depth, move_index, is_quiet, in_check, alpha, best, game_state,
        ) {
            unmake_move_in_place(game_state).map_err(|x| {
                MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
            })?;
            continue;
        }
        let lmr_reduction = lmr_reduction(depth, move_index, is_quiet, in_check);
        let use_pvs = should_use_pvs(depth, move_index, alpha, in_check);
        let score_opt = if !use_pvs {
            // Fallback to classic full-window search (v5 behavior).
            if lmr_reduction > 0 {
                let reduced_child = child.saturating_sub(lmr_reduction);
                let reduced = negamax(
                    game_state,
                    scorer,
                    reduced_child,
                    -alpha - 1,
                    -alpha,
                    ply.saturating_add(1),
                    child_allow_check_ext,
                    allow_null_pruning,
                    Some(mv),
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?;

                let Some(reduced_score) = reduced else {
                    unmake_move_in_place(game_state).map_err(|x| {
                        MoveGenerationError::InvalidState(format!(
                            "unmake_move_in_place failed: {x}"
                        ))
                    })?;
                    return Ok(None);
                };
                let reduced_score = -reduced_score;

                if reduced_score > alpha {
                    negamax(
                        game_state,
                        scorer,
                        child,
                        -beta,
                        -alpha,
                        ply.saturating_add(1),
                        child_allow_check_ext,
                        allow_null_pruning,
                        Some(mv),
                        nodes,
                        deadline,
                        node_cap,
                        stop_flag,
                        tt,
                        heuristics,
                    )?
                } else {
                    Some(-reduced_score)
                }
            } else {
                negamax(
                    game_state,
                    scorer,
                    child,
                    -beta,
                    -alpha,
                    ply.saturating_add(1),
                    child_allow_check_ext,
                    allow_null_pruning,
                    Some(mv),
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?
            }
        } else if move_index == 0 {
            // PV move: full-window search.
            negamax(
                game_state,
                scorer,
                child,
                -beta,
                -alpha,
                ply.saturating_add(1),
                child_allow_check_ext,
                allow_null_pruning,
                Some(mv),
                nodes,
                deadline,
                node_cap,
                stop_flag,
                tt,
                heuristics,
            )?
        } else {
            // Non-PV move: PVS zero-window probe first.
            let zero_window_opp_score = if lmr_reduction > 0 {
                let reduced_child = child.saturating_sub(lmr_reduction);
                negamax(
                    game_state,
                    scorer,
                    reduced_child,
                    -alpha - 1,
                    -alpha,
                    ply.saturating_add(1),
                    child_allow_check_ext,
                    allow_null_pruning,
                    Some(mv),
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?
            } else {
                negamax(
                    game_state,
                    scorer,
                    child,
                    -alpha - 1,
                    -alpha,
                    ply.saturating_add(1),
                    child_allow_check_ext,
                    allow_null_pruning,
                    Some(mv),
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?
            };

            let Some(pvs_score) = zero_window_opp_score else {
                unmake_move_in_place(game_state).map_err(|x| {
                    MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
                })?;
                return Ok(None);
            };
            let pvs_score_us = -pvs_score;

            if pvs_score_us > alpha {
                // Likely improves PV: confirm with full window.
                negamax(
                    game_state,
                    scorer,
                    child,
                    -beta,
                    -alpha,
                    ply.saturating_add(1),
                    child_allow_check_ext,
                    allow_null_pruning,
                    Some(mv),
                    nodes,
                    deadline,
                    node_cap,
                    stop_flag,
                    tt,
                    heuristics,
                )?
            } else {
                Some(pvs_score)
            }
        };

        unmake_move_in_place(game_state).map_err(|x| {
            MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
        })?;

        let Some(score) = score_opt else {
            return Ok(None);
        };
        let score = -score;

        if score > best {
            best = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            if is_quiet {
                heuristics.record_killer(ply_idx, mv);
                heuristics.record_history(game_state.side_to_move, mv, depth);
                heuristics.record_countermove(prev_move, mv);
                heuristics.record_continuation(game_state.side_to_move, prev_move, mv, depth);
            }
            break;
        }
    }

    let bound = if best <= alpha_orig {
        Bound::Upper
    } else if best >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    tt.store(TTEntry {
        key: game_state.zobrist_key,
        depth,
        score: tt_score_for_storage(best, ply),
        bound,
        best_move,
    });

    Ok(Some(best))
}

#[inline]
fn should_use_pvs(depth: u8, move_index: usize, alpha: i32, in_check: bool) -> bool {
    if in_check || depth < 3 || move_index == 0 {
        return false;
    }
    // Avoid PVS when alpha is still near the initial floor; in those nodes it
    // often causes extra full re-searches with little cutoff benefit.
    alpha > (-MATE_SCORE + 2000)
}

fn terminal_score(game_state: &GameState, ply: u8) -> i32 {
    if is_king_in_check(game_state, game_state.side_to_move) {
        -MATE_SCORE + i32::from(ply)
    } else {
        0
    }
}

#[inline]
fn tt_score_for_storage(score: i32, ply: u8) -> i32 {
    if score >= MATE_TT_THRESHOLD {
        score.saturating_add(i32::from(ply))
    } else if score <= -MATE_TT_THRESHOLD {
        score.saturating_sub(i32::from(ply))
    } else {
        score
    }
}

#[inline]
fn tt_score_from_storage(score: i32, ply: u8) -> i32 {
    if score >= MATE_TT_THRESHOLD {
        score.saturating_sub(i32::from(ply))
    } else if score <= -MATE_TT_THRESHOLD {
        score.saturating_add(i32::from(ply))
    } else {
        score
    }
}

fn quiescence<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    mut alpha: i32,
    beta: i32,
    qply: u8,
    nodes: &mut u64,
    deadline: Option<Instant>,
    node_cap: Option<u64>,
    stop_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
) -> MoveGenResult<Option<i32>> {
    if should_abort(deadline, stop_flag, *nodes, node_cap) {
        return Ok(None);
    }

    if is_draw_state(game_state) {
        return Ok(Some(repetition_draw_score(scorer.score(game_state))));
    }

    *nodes += 1;
    let in_check = is_king_in_check(game_state, game_state.side_to_move);

    // If side-to-move is in check, stand-pat is invalid.
    if in_check {
        let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
        if moves.is_empty() {
            return Ok(Some(terminal_score(game_state, qply)));
        }
        order_moves_basic(&mut moves, None);

        let mut local_alpha = alpha;
        for mv in moves {
            make_move_in_place(game_state, mv).map_err(|x| {
                MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
            })?;

            let score_opt = quiescence(
                game_state,
                scorer,
                -beta,
                -local_alpha,
                qply.saturating_add(1),
                nodes,
                deadline,
                node_cap,
                stop_flag,
            )?;

            unmake_move_in_place(game_state).map_err(|x| {
                MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
            })?;

            let Some(score) = score_opt else {
                return Ok(None);
            };
            let score = -score;

            if score >= beta {
                return Ok(Some(score));
            }
            if score > local_alpha {
                local_alpha = score;
            }
        }
        return Ok(Some(local_alpha));
    }

    let stand_pat = scorer.score(game_state);
    if stand_pat >= beta {
        return Ok(Some(stand_pat));
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }
    if qply >= QUIESCENCE_MAX_PLY {
        return Ok(Some(alpha));
    }

    let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
    if moves.is_empty() {
        return Ok(Some(terminal_score(game_state, qply)));
    }

    moves.retain(|m| is_tactical_move(*m));
    moves.retain(|m| passes_quiescence_pruning(*m, stand_pat, alpha, qply, game_state));
    if qply > 0 && qply < QUIESCENCE_CHECK_PLY {
        append_quiescence_check_moves(game_state, &mut moves)?;
    }
    order_moves_basic(&mut moves, None);

    for mv in moves {
        if should_abort(deadline, stop_flag, *nodes, node_cap) {
            return Ok(None);
        }

        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;

        let score_opt = quiescence(
            game_state,
            scorer,
            -beta,
            -alpha,
            qply.saturating_add(1),
            nodes,
            deadline,
            node_cap,
            stop_flag,
        )?;

        unmake_move_in_place(game_state).map_err(|x| {
            MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
        })?;

        let Some(score) = score_opt else {
            return Ok(None);
        };
        let score = -score;

        if score >= beta {
            return Ok(Some(score));
        }
        if score > alpha {
            alpha = score;
        }
    }

    Ok(Some(alpha))
}

#[inline]
fn is_draw_state(game_state: &GameState) -> bool {
    if game_state.halfmove_clock >= 100 {
        return true;
    }
    let current = game_state.zobrist_key;
    // Repetition can only occur along same side-to-move parity and only since
    // the last irreversible move (bounded by halfmove clock).
    let max_scan = usize::from(game_state.halfmove_clock)
        .saturating_add(1)
        .min(game_state.repetition_history.len());
    let mut count = 0usize;
    for h in game_state
        .repetition_history
        .iter()
        .rev()
        .take(max_scan)
        .step_by(2)
    {
        if *h == current {
            count += 1;
            if count >= 3 {
                return true;
            }
        }
    }
    false
}

#[inline]
fn repetition_draw_score(static_eval_side_to_move: i32) -> i32 {
    // V15:
    // - Non-zero contempt in near-equal positions to avoid passive draw lines.
    // - Much stronger draw penalties when clearly winning.
    // - Mild draw-seeking when clearly worse.
    const CLEAR_WIN_MARGIN: i32 = 180;
    const CLEAR_LOSS_MARGIN: i32 = -180;
    const CONTEMPT_CP: i32 = 18;
    const DRAW_PENALTY_BASE: i32 = 160;
    const DRAW_PENALTY_SCALE_DIV: i32 = 2;
    const DRAW_PENALTY_CAP: i32 = 1200;
    const DRAW_BONUS_BASE: i32 = 45;
    const DRAW_BONUS_SCALE_DIV: i32 = 8;
    const DRAW_BONUS_CAP: i32 = 240;

    if static_eval_side_to_move >= CLEAR_WIN_MARGIN {
        let mut penalty = DRAW_PENALTY_BASE
            + (static_eval_side_to_move / DRAW_PENALTY_SCALE_DIV).min(DRAW_PENALTY_CAP);
        if static_eval_side_to_move > 900 {
            penalty += 200;
        }
        return -penalty;
    }

    if static_eval_side_to_move <= CLEAR_LOSS_MARGIN {
        let bonus = DRAW_BONUS_BASE
            + ((-static_eval_side_to_move) / DRAW_BONUS_SCALE_DIV).min(DRAW_BONUS_CAP);
        return bonus;
    }

    if static_eval_side_to_move >= 0 {
        -CONTEMPT_CP
    } else {
        CONTEMPT_CP / 2
    }
}

#[inline]
fn child_depth(
    depth: u8,
    game_state: &GameState,
    allow_check_extension: bool,
    last_move: u64,
) -> u8 {
    let base = depth.saturating_sub(1);
    if should_extend_endgame_move(base, game_state, allow_check_extension, last_move) {
        base.saturating_add(1)
    } else {
        base
    }
}

#[inline]
fn child_allows_check_extension(
    depth: u8,
    game_state: &GameState,
    allow_check_extension: bool,
    last_move: u64,
) -> bool {
    allow_check_extension
        && !should_extend_endgame_move(
            depth.saturating_sub(1),
            game_state,
            allow_check_extension,
            last_move,
        )
}

#[inline]
fn should_extend_endgame_move(
    base_child_depth: u8,
    game_state: &GameState,
    allow_check_extension: bool,
    last_move: u64,
) -> bool {
    if !allow_check_extension {
        return false;
    }
    if base_child_depth > 1 {
        return false;
    }
    if !is_late_endgame(game_state) {
        return false;
    }
    // After make_move_in_place(), side_to_move has flipped. If that side is in check,
    // the move that was just made is checking.
    if is_king_in_check(game_state, game_state.side_to_move) {
        return true;
    }

    let moved_piece = piece_kind_from_code(move_moved_piece_code(last_move));
    let moved_color = game_state.side_to_move.opposite();
    let to_sq = move_to(last_move);

    if moved_piece == Some(crate::game_state::chess_types::PieceKind::Pawn)
        && is_advanced_pawn_square(moved_color, to_sq)
        && is_passed_pawn_on_square(game_state, moved_color, to_sq)
    {
        return true;
    }

    if is_king_pawn_only_endgame(game_state)
        && matches!(
            moved_piece,
            Some(crate::game_state::chess_types::PieceKind::King)
                | Some(crate::game_state::chess_types::PieceKind::Pawn)
        )
        && king_pawn_race_like_position(game_state, moved_color, to_sq)
    {
        return true;
    }

    false
}

#[inline]
fn is_late_endgame(game_state: &GameState) -> bool {
    let minor_phase = 1i32;
    let rook_phase = 2i32;
    let queen_phase = 4i32;
    let mut phase = 0i32;

    for color in [
        crate::game_state::chess_types::Color::Light,
        crate::game_state::chess_types::Color::Dark,
    ] {
        phase += (game_state.pieces[color.index()]
            [crate::game_state::chess_types::PieceKind::Knight.index()]
        .count_ones() as i32)
            * minor_phase;
        phase += (game_state.pieces[color.index()]
            [crate::game_state::chess_types::PieceKind::Bishop.index()]
        .count_ones() as i32)
            * minor_phase;
        phase += (game_state.pieces[color.index()]
            [crate::game_state::chess_types::PieceKind::Rook.index()]
        .count_ones() as i32)
            * rook_phase;
        phase += (game_state.pieces[color.index()]
            [crate::game_state::chess_types::PieceKind::Queen.index()]
        .count_ones() as i32)
            * queen_phase;
    }

    // Maximum phase here is 24. Treat <= 8 as late endgame.
    phase <= 8
}

#[inline]
fn is_advanced_pawn_square(color: crate::game_state::chess_types::Color, sq: u8) -> bool {
    let rank = sq / 8;
    match color {
        crate::game_state::chess_types::Color::Light => rank >= 5,
        crate::game_state::chess_types::Color::Dark => rank <= 2,
    }
}

fn is_passed_pawn_on_square(
    game_state: &GameState,
    color: crate::game_state::chess_types::Color,
    sq: u8,
) -> bool {
    let enemy_pawns = game_state.pieces[color.opposite().index()]
        [crate::game_state::chess_types::PieceKind::Pawn.index()];
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;

    for f in [file - 1, file, file + 1] {
        if !(0..=7).contains(&f) {
            continue;
        }
        match color {
            crate::game_state::chess_types::Color::Light => {
                let mut r = rank + 1;
                while r <= 7 {
                    let target = (r as u8) * 8 + (f as u8);
                    if (enemy_pawns & (1u64 << target)) != 0 {
                        return false;
                    }
                    r += 1;
                }
            }
            crate::game_state::chess_types::Color::Dark => {
                let mut r = rank - 1;
                while r >= 0 {
                    let target = (r as u8) * 8 + (f as u8);
                    if (enemy_pawns & (1u64 << target)) != 0 {
                        return false;
                    }
                    r -= 1;
                }
            }
        }
    }
    true
}

#[inline]
fn is_king_pawn_only_endgame(game_state: &GameState) -> bool {
    for color in [
        crate::game_state::chess_types::Color::Light,
        crate::game_state::chess_types::Color::Dark,
    ] {
        let idx = color.index();
        if game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Queen.index()] != 0
            || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Rook.index()] != 0
            || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Bishop.index()]
                != 0
            || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Knight.index()]
                != 0
        {
            return false;
        }
    }
    true
}

fn king_pawn_race_like_position(
    game_state: &GameState,
    moved_color: crate::game_state::chess_types::Color,
    moved_to_sq: u8,
) -> bool {
    let opp = moved_color.opposite();
    let own_king_bb = game_state.pieces[moved_color.index()]
        [crate::game_state::chess_types::PieceKind::King.index()];
    let opp_king_bb =
        game_state.pieces[opp.index()][crate::game_state::chess_types::PieceKind::King.index()];
    if own_king_bb == 0 || opp_king_bb == 0 {
        return false;
    }
    let own_king_sq = own_king_bb.trailing_zeros() as u8;
    let opp_king_sq = opp_king_bb.trailing_zeros() as u8;

    let kings_close = chebyshev_distance(own_king_sq, opp_king_sq) <= 2;
    let move_near_promo = is_advanced_pawn_square(moved_color, moved_to_sq);
    kings_close || move_near_promo
}

#[inline]
fn chebyshev_distance(a: u8, b: u8) -> i32 {
    let af = i32::from(a % 8);
    let ar = i32::from(a / 8);
    let bf = i32::from(b % 8);
    let br = i32::from(b / 8);
    (af - bf).abs().max((ar - br).abs())
}

#[inline]
fn is_tactical_move(move_description: u64) -> bool {
    (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) != 0
        || move_promotion_piece_code(move_description) != NO_PIECE_CODE
}

fn order_moves(
    moves: &mut [u64],
    tt_move: Option<u64>,
    prev_move: Option<u64>,
    killers: [u64; 2],
    heuristics: &SearchHeuristics,
    side_to_move: crate::game_state::chess_types::Color,
) {
    moves.sort_unstable_by_key(|m| {
        -move_order_score(*m, tt_move, prev_move, killers, heuristics, side_to_move)
    });
}

fn order_moves_basic(moves: &mut [u64], tt_move: Option<u64>) {
    moves.sort_unstable_by_key(|m| -move_order_score_basic(*m, tt_move));
}

fn move_order_score(
    move_description: u64,
    tt_move: Option<u64>,
    prev_move: Option<u64>,
    killers: [u64; 2],
    heuristics: &SearchHeuristics,
    side_to_move: crate::game_state::chess_types::Color,
) -> i32 {
    let mut score = move_order_score_basic(move_description, tt_move);
    if is_quiet_move(move_description) {
        if move_description == killers[0] {
            score += 80_000;
        } else if move_description == killers[1] {
            score += 70_000;
        }

        if let Some(piece) = piece_kind_from_code(move_moved_piece_code(move_description)) {
            let to = move_to_square(move_description);
            score += heuristics.history[side_to_move.index()][piece.index()][to] / 2;
            score += heuristics.continuation_bonus(side_to_move, prev_move, piece, to) / 2;
        }

        if heuristics.is_countermove(prev_move, move_description) {
            score += 60_000;
        }
    }
    score
}

fn move_order_score_basic(move_description: u64, tt_move: Option<u64>) -> i32 {
    if Some(move_description) == tt_move {
        return 1_000_000;
    }
    let mut score = 0i32;
    if (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) != 0 {
        let victim = capture_value(move_description);
        let aggressor = piece_kind_from_code(move_moved_piece_code(move_description))
            .map(piece_value)
            .unwrap_or(100);
        let see = static_exchange_estimate(move_description);
        // Tactical ordering blend:
        // - MVV/LVA preference (high victim, low aggressor)
        // - SEE bonus to push likely winning captures earlier
        score += 100_000 + (victim * 16) - aggressor + (see * 4);
    }
    if move_promotion_piece_code(move_description) != NO_PIECE_CODE {
        let promo_value = piece_kind_from_code(move_promotion_piece_code(move_description))
            .map(piece_value)
            .unwrap_or(0);
        score += 90_000 + promo_value;
    }
    score
}

#[inline]
fn passes_quiescence_pruning(
    move_description: u64,
    stand_pat: i32,
    alpha: i32,
    qply: u8,
    game_state: &GameState,
) -> bool {
    if move_promotion_piece_code(move_description) != NO_PIECE_CODE {
        return true;
    }

    let margin = quiescence_delta_margin(qply, game_state);
    let max_gain = capture_value(move_description) + promotion_gain(move_description);
    if stand_pat + max_gain + margin < alpha {
        return false;
    }

    if (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) != 0
        && static_exchange_estimate(move_description) < see_bad_capture_threshold(qply, game_state)
    {
        return false;
    }

    true
}

#[inline]
fn see_bad_capture_threshold(qply: u8, game_state: &GameState) -> i32 {
    let mut threshold = SEE_BAD_CAPTURE_THRESHOLD + (i32::from(qply) * 20);
    if is_critical_endgame(game_state) {
        // Endgames are tactical and tempo-sensitive: allow more speculative captures.
        threshold -= 90;
    } else if is_late_endgame(game_state) {
        threshold -= 40;
    }
    threshold
}

#[inline]
fn quiescence_delta_margin(qply: u8, game_state: &GameState) -> i32 {
    let mut margin = QUIESCENCE_DELTA_MARGIN.saturating_sub(i32::from(qply) * 10);
    if is_critical_endgame(game_state) {
        // Increase margin to reduce over-pruning in king/pawn races.
        margin += 100;
    } else if is_late_endgame(game_state) {
        margin += 50;
    }
    margin
}

fn append_quiescence_check_moves(
    game_state: &mut GameState,
    moves: &mut Vec<u64>,
) -> MoveGenResult<()> {
    let mut seen = [false; 4096];
    for &mv in moves.iter() {
        let key = ((usize::from(crate::moves::move_descriptions::move_from(mv))) << 6)
            | usize::from(crate::moves::move_descriptions::move_to(mv));
        seen[key] = true;
    }
    let all = generate_legal_move_descriptions_in_place(game_state)?;
    for mv in all {
        let key = ((usize::from(crate::moves::move_descriptions::move_from(mv))) << 6)
            | usize::from(crate::moves::move_descriptions::move_to(mv));
        if is_tactical_move(mv) || seen[key] {
            continue;
        }
        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;
        let gives_check = is_king_in_check(game_state, game_state.side_to_move);
        unmake_move_in_place(game_state).map_err(|x| {
            MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
        })?;
        if gives_check {
            moves.push(mv);
            seen[key] = true;
        }
    }
    Ok(())
}

#[inline]
fn capture_value(move_description: u64) -> i32 {
    piece_kind_from_code(move_captured_piece_code(move_description))
        .map(piece_value)
        .unwrap_or(0)
}

#[inline]
fn promotion_gain(move_description: u64) -> i32 {
    piece_kind_from_code(move_promotion_piece_code(move_description))
        .map(|p| piece_value(p) - piece_value(crate::game_state::chess_types::PieceKind::Pawn))
        .unwrap_or(0)
}

#[inline]
fn static_exchange_estimate(move_description: u64) -> i32 {
    let victim = capture_value(move_description);
    let attacker = piece_kind_from_code(move_moved_piece_code(move_description))
        .map(piece_value)
        .unwrap_or(100);
    victim + promotion_gain(move_description) - attacker
}

#[inline]
fn move_to_square(move_description: u64) -> usize {
    crate::moves::move_descriptions::move_to(move_description) as usize
}

#[inline]
fn is_quiet_move(move_description: u64) -> bool {
    (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) == 0
        && move_promotion_piece_code(move_description) == NO_PIECE_CODE
}

#[inline]
fn lmr_reduction(depth: u8, move_index: usize, is_quiet: bool, in_check: bool) -> u8 {
    if !is_quiet || in_check || depth < 3 || move_index < 3 {
        0
    } else if depth >= 9 && move_index >= 12 {
        3
    } else if depth >= 7 && move_index >= 7 {
        2
    } else if depth >= 5 && move_index >= 5 {
        1
    } else if depth >= 6 && move_index >= 8 {
        2
    } else {
        1
    }
}

#[inline]
fn should_lmp_prune(
    depth: u8,
    move_index: usize,
    is_quiet: bool,
    in_check: bool,
    alpha: i32,
    best: i32,
    game_state: &GameState,
) -> bool {
    if !is_quiet || in_check {
        return false;
    }
    if is_critical_endgame(game_state) {
        // Avoid pruning quiet king/pawn moves in critical endgames.
        return false;
    }
    if depth > 3 {
        return false;
    }
    // Only prune once we have some evidence a good move already exists.
    if best <= -MATE_SCORE + 2000 || alpha <= -MATE_SCORE + 2000 {
        return false;
    }

    let threshold = match depth {
        0 | 1 => 3,
        2 => 6,
        3 => 10,
        _ => usize::MAX,
    };

    move_index >= threshold
}

#[inline]
fn is_critical_endgame(game_state: &GameState) -> bool {
    is_king_pawn_only_endgame(game_state) || is_late_endgame(game_state)
}

#[derive(Debug, Clone, Copy)]
struct NullMoveUndo {
    prev_side_to_move: crate::game_state::chess_types::Color,
    prev_en_passant_square: Option<crate::game_state::chess_types::Square>,
    prev_halfmove_clock: u16,
    prev_ply: u16,
    prev_zobrist_key: u64,
    prev_repetition_len: usize,
}

#[inline]
fn should_try_null_move(depth: u8, in_check: bool, beta: i32, game_state: &GameState) -> bool {
    if in_check || depth < 4 {
        return false;
    }
    if beta > (MATE_SCORE - 1000) {
        return false;
    }
    if is_late_endgame(game_state) {
        return false;
    }
    has_non_pawn_material(game_state, game_state.side_to_move)
}

#[inline]
fn should_verify_null_cutoff(depth: u8, in_check: bool) -> bool {
    !in_check && depth >= 8
}

#[inline]
fn has_non_pawn_material(
    game_state: &GameState,
    color: crate::game_state::chess_types::Color,
) -> bool {
    let idx = color.index();
    game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Knight.index()] != 0
        || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Bishop.index()] != 0
        || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Rook.index()] != 0
        || game_state.pieces[idx][crate::game_state::chess_types::PieceKind::Queen.index()] != 0
}

fn make_null_move(game_state: &mut GameState) -> NullMoveUndo {
    use crate::search::zobrist::{en_passant_file_key, side_to_move_key};
    let undo = NullMoveUndo {
        prev_side_to_move: game_state.side_to_move,
        prev_en_passant_square: game_state.en_passant_square,
        prev_halfmove_clock: game_state.halfmove_clock,
        prev_ply: game_state.ply,
        prev_zobrist_key: game_state.zobrist_key,
        prev_repetition_len: game_state.repetition_history.len(),
    };

    if let Some(ep) = game_state.en_passant_square {
        game_state.zobrist_key ^= en_passant_file_key(ep % 8);
    }
    game_state.en_passant_square = None;
    game_state.side_to_move = game_state.side_to_move.opposite();
    game_state.zobrist_key ^= side_to_move_key();
    game_state.halfmove_clock = game_state.halfmove_clock.saturating_add(1);
    game_state.ply = game_state.ply.saturating_add(1);
    game_state.repetition_history.push(game_state.zobrist_key);

    undo
}

fn unmake_null_move(game_state: &mut GameState, undo: NullMoveUndo) {
    game_state.side_to_move = undo.prev_side_to_move;
    game_state.en_passant_square = undo.prev_en_passant_square;
    game_state.halfmove_clock = undo.prev_halfmove_clock;
    game_state.ply = undo.prev_ply;
    game_state.zobrist_key = undo.prev_zobrist_key;
    game_state
        .repetition_history
        .truncate(undo.prev_repetition_len);
}

type HistoryTable = [[[i32; 64]; 6]; 2];
type CounterMoveTable = [[u64; 64]; 6];
type ContinuationHistoryTable = [[[[[i32; 64]; 6]; 64]; 6]; 2];

#[derive(Debug, Clone)]
struct SearchHeuristics {
    killers: [[u64; 2]; MAX_PLY],
    history: HistoryTable,
    countermove: CounterMoveTable,
    continuation_history: Box<ContinuationHistoryTable>,
}

impl Default for SearchHeuristics {
    fn default() -> Self {
        Self {
            killers: [[0; 2]; MAX_PLY],
            history: [[[0; 64]; 6]; 2],
            countermove: [[0; 64]; 6],
            continuation_history: Box::new([[[[[0; 64]; 6]; 64]; 6]; 2]),
        }
    }
}

impl SearchHeuristics {
    fn reset_iteration(&mut self) {
        self.killers.fill([0; 2]);
    }

    fn killers_at(&self, ply: usize) -> [u64; 2] {
        self.killers[ply]
    }

    fn record_killer(&mut self, ply: usize, mv: u64) {
        if self.killers[ply][0] == mv {
            return;
        }
        self.killers[ply][1] = self.killers[ply][0];
        self.killers[ply][0] = mv;
    }

    fn record_history(&mut self, side: crate::game_state::chess_types::Color, mv: u64, depth: u8) {
        let Some(piece) = piece_kind_from_code(move_moved_piece_code(mv)) else {
            return;
        };
        let to = move_to_square(mv);
        let bonus = i32::from(depth) * i32::from(depth);
        let entry = &mut self.history[side.index()][piece.index()][to];
        *entry = (*entry + bonus).min(50_000);
    }

    fn record_countermove(&mut self, prev_move: Option<u64>, mv: u64) {
        let Some((prev_piece, prev_to)) = move_meta(prev_move) else {
            return;
        };
        self.countermove[prev_piece.index()][prev_to] = mv;
    }

    fn is_countermove(&self, prev_move: Option<u64>, mv: u64) -> bool {
        let Some((prev_piece, prev_to)) = move_meta(prev_move) else {
            return false;
        };
        self.countermove[prev_piece.index()][prev_to] == mv
    }

    fn record_continuation(
        &mut self,
        side: crate::game_state::chess_types::Color,
        prev_move: Option<u64>,
        mv: u64,
        depth: u8,
    ) {
        let Some((prev_piece, prev_to)) = move_meta(prev_move) else {
            return;
        };
        let Some((piece, to)) = move_meta(Some(mv)) else {
            return;
        };
        let bonus = i32::from(depth) * i32::from(depth);
        let entry = &mut self.continuation_history[side.index()][prev_piece.index()][prev_to]
            [piece.index()][to];
        *entry = (*entry + bonus).min(50_000);
    }

    fn continuation_bonus(
        &self,
        side: crate::game_state::chess_types::Color,
        prev_move: Option<u64>,
        piece: crate::game_state::chess_types::PieceKind,
        to: usize,
    ) -> i32 {
        let Some((prev_piece, prev_to)) = move_meta(prev_move) else {
            return 0;
        };
        self.continuation_history[side.index()][prev_piece.index()][prev_to][piece.index()][to]
    }
}

#[inline]
fn move_meta(
    move_description: Option<u64>,
) -> Option<(crate::game_state::chess_types::PieceKind, usize)> {
    let mv = move_description?;
    let piece = piece_kind_from_code(move_moved_piece_code(mv))?;
    Some((piece, move_to_square(mv)))
}

#[inline]
fn piece_value(piece: crate::game_state::chess_types::PieceKind) -> i32 {
    match piece {
        crate::game_state::chess_types::PieceKind::Pawn => 100,
        crate::game_state::chess_types::PieceKind::Knight => 320,
        crate::game_state::chess_types::PieceKind::Bishop => 330,
        crate::game_state::chess_types::PieceKind::Rook => 500,
        crate::game_state::chess_types::PieceKind::Queen => 900,
        crate::game_state::chess_types::PieceKind::King => 20_000,
    }
}

pub fn principal_variation_from_tt(
    game_state: &GameState,
    tt: &mut TranspositionTable,
    max_depth: u8,
) -> PrincipalVariation {
    let mut pv = PrincipalVariation::default();
    let mut state = game_state.clone();

    for _ in 0..max_depth {
        let Some(entry) = tt.probe(state.zobrist_key) else {
            break;
        };
        let Some(best_move) = entry.best_move else {
            break;
        };
        let Ok(lan) = move_description_to_long_algebraic(best_move, &state) else {
            break;
        };
        if long_algebraic_to_move_description_checked(&lan, &state).is_none() {
            break;
        }
        pv.moves.push(best_move);
        if make_move_in_place(&mut state, best_move).is_err() {
            break;
        }
    }

    pv
}

fn long_algebraic_to_move_description_checked(lan: &str, game_state: &GameState) -> Option<u64> {
    crate::utils::long_algebraic::long_algebraic_to_move_description(lan, game_state).ok()
}

#[cfg(test)]
mod tests {
    use crate::move_generation::legal_move_generator::LegalMoveGenerator;
    use crate::search::board_scoring::MaterialScorer;
    use crate::utils::long_algebraic::move_description_to_long_algebraic;

    use super::{
        iterative_deepening_search, tt_score_for_storage, tt_score_from_storage, SearchConfig,
        MATE_SCORE,
    };
    use crate::game_state::game_state::GameState;

    #[test]
    fn search_depth_zero_returns_eval_only() {
        let game = GameState::new_game();
        let gen = LegalMoveGenerator;
        let scorer = MaterialScorer;

        let result = iterative_deepening_search(
            &game,
            &gen,
            &scorer,
            SearchConfig {
                max_depth: 0,
                ..SearchConfig::default()
            },
        )
        .expect("search should run");

        assert_eq!(result.best_move, None);
        assert_eq!(result.best_score, 0);
        assert_eq!(result.reached_depth, 0);
    }

    #[test]
    fn search_prefers_winning_capture_in_simple_position() {
        let game =
            GameState::from_fen("4k3/8/8/8/8/8/4q3/4KQ2 w - - 0 1").expect("FEN should parse");
        let gen = LegalMoveGenerator;
        let scorer = MaterialScorer;

        let result = iterative_deepening_search(
            &game,
            &gen,
            &scorer,
            SearchConfig {
                max_depth: 1,
                ..SearchConfig::default()
            },
        )
        .expect("search should run");

        let best_move = result.best_move.expect("best move should exist");
        let lan = move_description_to_long_algebraic(best_move, &game)
            .expect("LAN conversion should succeed");

        assert_eq!(lan, "f1e2");
    }

    #[test]
    fn search_finds_mate_in_one_at_depth_one() {
        use crate::move_generation::legal_move_apply::apply_move;
        use crate::move_generation::move_generator::MoveGenerator;

        let game =
            GameState::from_fen("6k1/5Q2/6K1/8/8/8/8/8 w - - 0 1").expect("FEN should parse");
        let gen = LegalMoveGenerator;
        let scorer = MaterialScorer;

        let result = iterative_deepening_search(
            &game,
            &gen,
            &scorer,
            SearchConfig {
                max_depth: 1,
                ..SearchConfig::default()
            },
        )
        .expect("search should run");

        let best_move = result.best_move.expect("best move should exist");
        let next = apply_move(&game, best_move).expect("best move should apply");
        let replies = gen
            .generate_legal_moves(&next)
            .expect("move generation should succeed");

        assert!(replies.is_empty(), "best move should deliver checkmate");
        assert!(
            result.best_score > 29000,
            "mate score should dominate material, got {}",
            result.best_score
        );
    }

    #[test]
    fn search_respects_node_cap() {
        let game = GameState::new_game();
        let gen = LegalMoveGenerator;
        let scorer = MaterialScorer;

        let result = iterative_deepening_search(
            &game,
            &gen,
            &scorer,
            SearchConfig {
                max_depth: 8,
                max_nodes: Some(200),
                ..SearchConfig::default()
            },
        )
        .expect("search should run");

        assert!(result.nodes <= 200, "nodes exceeded cap: {}", result.nodes);
    }

    #[test]
    fn mate_score_tt_roundtrip_is_consistent() {
        let ply = 7u8;
        let mate_win_score = MATE_SCORE - 12;
        let mate_loss_score = -MATE_SCORE + 9;

        let stored_win = tt_score_for_storage(mate_win_score, ply);
        let stored_loss = tt_score_for_storage(mate_loss_score, ply);

        assert_eq!(tt_score_from_storage(stored_win, ply), mate_win_score);
        assert_eq!(tt_score_from_storage(stored_loss, ply), mate_loss_score);
    }
}
