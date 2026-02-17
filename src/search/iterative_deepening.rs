//! Iterative deepening search with negamax alpha-beta pruning.
//!
//! Implements depth-progressive search that repeatedly refines best-move
//! output and supports configurable search depth limits.

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::move_generator::{MoveGenResult, MoveGenerator};
use crate::search::board_scoring::BoardScorer;

const MATE_SCORE: i32 = 30000;

#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    pub max_depth: u8,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { max_depth: 4 }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchResult {
    pub best_move: Option<u64>,
    pub best_score: i32,
    pub reached_depth: u8,
    pub nodes: u64,
}

pub fn iterative_deepening_search<G: MoveGenerator, S: BoardScorer>(
    game_state: &GameState,
    generator: &G,
    scorer: &S,
    config: SearchConfig,
) -> MoveGenResult<SearchResult> {
    if config.max_depth == 0 {
        return Ok(SearchResult {
            best_move: None,
            best_score: scorer.score(game_state),
            reached_depth: 0,
            nodes: 1,
        });
    }

    let mut result = SearchResult::default();

    for depth in 1..=config.max_depth {
        let mut nodes = 0u64;
        let (best_move, best_score) =
            negamax_root(game_state, generator, scorer, depth, &mut nodes)?;

        result.best_move = best_move;
        result.best_score = best_score;
        result.reached_depth = depth;
        result.nodes = nodes;
    }

    Ok(result)
}

fn negamax_root<G: MoveGenerator, S: BoardScorer>(
    game_state: &GameState,
    generator: &G,
    scorer: &S,
    depth: u8,
    nodes: &mut u64,
) -> MoveGenResult<(Option<u64>, i32)> {
    let moves = generator.generate_legal_moves(game_state)?;
    if moves.is_empty() {
        let score = terminal_score(game_state, 0);
        *nodes += 1;
        return Ok((None, score));
    }

    let mut alpha = -MATE_SCORE;
    let beta = MATE_SCORE;
    let mut best_move = None;
    let mut best_score = -MATE_SCORE;

    for mv in moves {
        let score = -negamax(
            &mv.game_after_move,
            generator,
            scorer,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            1,
            nodes,
        )?;

        if score > best_score {
            best_score = score;
            best_move = Some(mv.move_description);
        }
        if score > alpha {
            alpha = score;
        }
    }

    Ok((best_move, best_score))
}

fn negamax<G: MoveGenerator, S: BoardScorer>(
    game_state: &GameState,
    generator: &G,
    scorer: &S,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    nodes: &mut u64,
) -> MoveGenResult<i32> {
    *nodes += 1;

    if depth == 0 {
        // Even at horizon, terminal positions must dominate material so the
        // engine reliably chooses mating lines (for example, mate in 1).
        let horizon_moves = generator.generate_legal_moves(game_state)?;
        if horizon_moves.is_empty() {
            return Ok(terminal_score(game_state, ply));
        }
        return Ok(scorer.score(game_state));
    }

    let moves = generator.generate_legal_moves(game_state)?;
    if moves.is_empty() {
        return Ok(terminal_score(game_state, ply));
    }

    let mut best = -MATE_SCORE;

    for mv in moves {
        let score = -negamax(
            &mv.game_after_move,
            generator,
            scorer,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            ply.saturating_add(1),
            nodes,
        )?;

        if score > best {
            best = score;
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            break;
        }
    }

    Ok(best)
}

fn terminal_score(game_state: &GameState, ply: u8) -> i32 {
    if is_king_in_check(game_state, game_state.side_to_move) {
        -MATE_SCORE + i32::from(ply)
    } else {
        0
    }
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

        let result =
            iterative_deepening_search(&game, &gen, &scorer, SearchConfig { max_depth: 0 })
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

        let result =
            iterative_deepening_search(&game, &gen, &scorer, SearchConfig { max_depth: 1 })
                .expect("search should run");

        let best_move = result.best_move.expect("best move should exist");
        let lan = move_description_to_long_algebraic(best_move, &game)
            .expect("LAN conversion should succeed");

        assert_eq!(lan, "f1e2");
    }

    #[test]
    fn search_fails_gracefully_when_movegen_errors() {
        use crate::move_generation::move_generator::{
            MoveGenResult, MoveGenerationError, MoveGenerator, NullMoveGenerator,
        };

        fn run_with_null<G: MoveGenerator>(generator: &G) -> MoveGenResult<()> {
            let game = GameState::new_game();
            let scorer = MaterialScorer;
            let _ = iterative_deepening_search(
                &game,
                generator,
                &scorer,
                SearchConfig { max_depth: 1 },
            )?;
            Ok(())
        }

        let null = NullMoveGenerator;
        let err = run_with_null(&null).expect_err("null move generator should error");
        assert!(matches!(err, MoveGenerationError::NotImplemented));
    }

    #[test]
    fn search_finds_mate_in_one_at_depth_one() {
        use crate::move_generation::legal_move_apply::apply_move;
        use crate::move_generation::move_generator::MoveGenerator;

        let game =
            GameState::from_fen("6k1/5Q2/6K1/8/8/8/8/8 w - - 0 1").expect("FEN should parse");
        let gen = LegalMoveGenerator;
        let scorer = MaterialScorer;

        let result =
            iterative_deepening_search(&game, &gen, &scorer, SearchConfig { max_depth: 1 })
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
