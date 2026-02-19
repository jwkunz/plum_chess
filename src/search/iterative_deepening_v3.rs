//! Iterative deepening search with negamax alpha-beta pruning (V3).
//!
//! Implements depth-progressive search that repeatedly refines best-move
//! output and supports configurable search depth limits.
//!
//! V3 heuristics:
//! - Repetition-while-winning draw penalty.
//! - Late-endgame check extension.

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::{make_move_in_place, unmake_move_in_place};
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::move_generation::move_generator::{MoveGenResult, MoveGenerationError, MoveGenerator};
use crate::moves::move_descriptions::{
    move_captured_piece_code, move_promotion_piece_code, piece_kind_from_code, FLAG_CAPTURE,
    FLAG_EN_PASSANT, NO_PIECE_CODE,
};
use crate::search::board_scoring::BoardScorer;
use crate::search::transposition_table::{Bound, TTEntry, TTStats, TranspositionTable};
use crate::utils::long_algebraic::move_description_to_long_algebraic;
use std::time::{Duration, Instant};

const MATE_SCORE: i32 = 30000;

#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    pub max_depth: u8,
    pub movetime_ms: Option<u64>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: 4,
            movetime_ms: None,
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

    for depth in 1..=config.max_depth {
        if let Some(limit) = deadline {
            if Instant::now() >= limit {
                break;
            }
        }

        let mut nodes = 0u64;
        let mut root_state = game_state.clone();
        let Some((best_move, best_score)) =
            negamax_root(&mut root_state, scorer, depth, &mut nodes, deadline, tt)?
        else {
            break;
        };

        result.best_move = best_move;
        result.best_score = best_score;
        result.reached_depth = depth;
        result.nodes = nodes;
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

fn negamax_root<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    depth: u8,
    nodes: &mut u64,
    deadline: Option<Instant>,
    tt: &mut TranspositionTable,
) -> MoveGenResult<Option<(Option<u64>, i32)>> {
    let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
    if moves.is_empty() {
        let score = terminal_score(game_state, 0);
        *nodes += 1;
        return Ok(Some((None, score)));
    }

    let tt_move = tt.probe(game_state.zobrist_key).and_then(|e| e.best_move);
    order_moves(&mut moves, tt_move);

    let mut alpha = -MATE_SCORE;
    let beta = MATE_SCORE;
    let mut best_move = None;
    let mut best_score = -MATE_SCORE;

    for mv in moves {
        if let Some(limit) = deadline {
            if Instant::now() >= limit {
                return Ok(None);
            }
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
            nodes,
            deadline,
            tt,
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
    }

    Ok(Some((best_move, best_score)))
}

fn negamax<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    allow_check_extension: bool,
    nodes: &mut u64,
    deadline: Option<Instant>,
    tt: &mut TranspositionTable,
) -> MoveGenResult<Option<i32>> {
    if let Some(limit) = deadline {
        if Instant::now() >= limit {
            return Ok(None);
        }
    }

    if is_draw_state(game_state) {
        return Ok(Some(repetition_draw_score(scorer.score(game_state))));
    }

    let alpha_orig = alpha;

    if let Some(entry) = tt.probe(game_state.zobrist_key) {
        if entry.depth >= depth {
            match entry.bound {
                Bound::Exact => return Ok(Some(entry.score)),
                Bound::Lower if entry.score >= beta => return Ok(Some(entry.score)),
                Bound::Upper if entry.score <= alpha => return Ok(Some(entry.score)),
                _ => {}
            }
        }
    }

    *nodes += 1;

    if depth == 0 {
        return quiescence(game_state, scorer, alpha, beta, nodes, deadline);
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
    order_moves(&mut moves, tt_move);

    let mut best = -MATE_SCORE;
    let mut best_move: Option<u64> = None;

    for mv in moves {
        if let Some(limit) = deadline {
            if Instant::now() >= limit {
                return Ok(None);
            }
        }

        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;

        let score_opt = negamax(
            game_state,
            scorer,
            child_depth(depth, game_state, allow_check_extension),
            -beta,
            -alpha,
            ply.saturating_add(1),
            child_allows_check_extension(depth, game_state, allow_check_extension),
            nodes,
            deadline,
            tt,
        )?;

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
        score: best,
        bound,
        best_move,
    });

    Ok(Some(best))
}

fn terminal_score(game_state: &GameState, ply: u8) -> i32 {
    if is_king_in_check(game_state, game_state.side_to_move) {
        -MATE_SCORE + i32::from(ply)
    } else {
        0
    }
}

fn quiescence<S: BoardScorer>(
    game_state: &mut GameState,
    scorer: &S,
    mut alpha: i32,
    beta: i32,
    nodes: &mut u64,
    deadline: Option<Instant>,
) -> MoveGenResult<Option<i32>> {
    if let Some(limit) = deadline {
        if Instant::now() >= limit {
            return Ok(None);
        }
    }

    if is_draw_state(game_state) {
        return Ok(Some(repetition_draw_score(scorer.score(game_state))));
    }

    *nodes += 1;

    // If side-to-move is in check, stand-pat is invalid.
    if is_king_in_check(game_state, game_state.side_to_move) {
        let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
        if moves.is_empty() {
            return Ok(Some(terminal_score(game_state, 0)));
        }
        order_moves(&mut moves, None);

        let mut local_alpha = alpha;
        for mv in moves {
            make_move_in_place(game_state, mv).map_err(|x| {
                MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
            })?;

            let score_opt = quiescence(game_state, scorer, -beta, -local_alpha, nodes, deadline)?;

            unmake_move_in_place(game_state).map_err(|x| {
                MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
            })?;

            let Some(score) = score_opt else {
                return Ok(None);
            };
            let score = -score;

            if score >= beta {
                return Ok(Some(beta));
            }
            if score > local_alpha {
                local_alpha = score;
            }
        }
        return Ok(Some(local_alpha));
    }

    let stand_pat = scorer.score(game_state);
    if stand_pat >= beta {
        return Ok(Some(beta));
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let mut moves = generate_legal_move_descriptions_in_place(game_state)?;
    if moves.is_empty() {
        return Ok(Some(terminal_score(game_state, 0)));
    }

    moves.retain(|m| is_tactical_move(*m));
    order_moves(&mut moves, None);

    for mv in moves {
        if let Some(limit) = deadline {
            if Instant::now() >= limit {
                return Ok(None);
            }
        }

        make_move_in_place(game_state, mv).map_err(|x| {
            MoveGenerationError::InvalidState(format!("make_move_in_place failed: {x}"))
        })?;

        let score_opt = quiescence(game_state, scorer, -beta, -alpha, nodes, deadline)?;

        unmake_move_in_place(game_state).map_err(|x| {
            MoveGenerationError::InvalidState(format!("unmake_move_in_place failed: {x}"))
        })?;

        let Some(score) = score_opt else {
            return Ok(None);
        };
        let score = -score;

        if score >= beta {
            return Ok(Some(beta));
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
    let count = game_state
        .repetition_history
        .iter()
        .filter(|h| **h == current)
        .count();
    count >= 3
}

#[inline]
fn repetition_draw_score(static_eval_side_to_move: i32) -> i32 {
    const DRAW_PENALTY_BASE: i32 = 40;
    const DRAW_PENALTY_CAP: i32 = 220;
    const WINNING_MARGIN: i32 = 80;

    if static_eval_side_to_move > WINNING_MARGIN {
        let penalty = DRAW_PENALTY_BASE + (static_eval_side_to_move / 4).min(DRAW_PENALTY_CAP);
        -penalty
    } else if static_eval_side_to_move < -WINNING_MARGIN {
        let bonus = DRAW_PENALTY_BASE + ((-static_eval_side_to_move) / 4).min(DRAW_PENALTY_CAP);
        bonus
    } else {
        0
    }
}

#[inline]
fn child_depth(depth: u8, game_state: &GameState, allow_check_extension: bool) -> u8 {
    let base = depth.saturating_sub(1);
    if should_extend_check(base, game_state, allow_check_extension) {
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
) -> bool {
    allow_check_extension
        && !should_extend_check(depth.saturating_sub(1), game_state, allow_check_extension)
}

#[inline]
fn should_extend_check(
    base_child_depth: u8,
    game_state: &GameState,
    allow_check_extension: bool,
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
    is_king_in_check(game_state, game_state.side_to_move)
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
fn is_tactical_move(move_description: u64) -> bool {
    (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) != 0
        || move_promotion_piece_code(move_description) != NO_PIECE_CODE
}

fn order_moves(moves: &mut [u64], tt_move: Option<u64>) {
    moves.sort_by_key(|m| -move_order_score(*m, tt_move));
}

fn move_order_score(move_description: u64, tt_move: Option<u64>) -> i32 {
    if Some(move_description) == tt_move {
        return 1_000_000;
    }
    let mut score = 0i32;
    if (move_description & (FLAG_CAPTURE | FLAG_EN_PASSANT)) != 0 {
        let victim = piece_kind_from_code(move_captured_piece_code(move_description))
            .map(piece_value)
            .unwrap_or(100);
        score += 100_000 + victim;
    }
    if move_promotion_piece_code(move_description) != NO_PIECE_CODE {
        score += 90_000;
    }
    score
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
        let Ok(next) = crate::move_generation::legal_move_apply::apply_move(&state, best_move)
        else {
            break;
        };
        state = next;
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

    use super::{iterative_deepening_search, SearchConfig};
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
}
