//! Iterative-deepening material-search engine.
//!
//! Wraps the core negamax alpha-beta search with fixed-depth configuration and
//! material scoring for deterministic stronger difficulty levels.

use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::LegalMoveGenerator;
use crate::move_generation::move_generator::MoveGenerator;
use crate::search::board_scoring::MaterialScorer;
use crate::search::iterative_deepening::{iterative_deepening_search, SearchConfig};
use crate::tables::opening_book::OpeningBook;
use rand::rng;

pub struct IterativeEngine {
    default_depth: u8,
    move_generator: LegalMoveGenerator,
    scorer: MaterialScorer,
    opening_book: OpeningBook,
}

impl IterativeEngine {
    pub fn new(default_depth: u8) -> Self {
        Self {
            default_depth,
            move_generator: LegalMoveGenerator,
            scorer: MaterialScorer,
            opening_book: OpeningBook::load_default(),
        }
    }
}

impl Engine for IterativeEngine {
    fn name(&self) -> &str {
        "PlumChess Iterative"
    }

    fn author(&self) -> &str {
        "jwkunz+codex"
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        if params.depth.is_none() && game_state.ply < 20 {
            let mut rng = rng();
            if let Some(book_move) = self.opening_book.choose_weighted_move(game_state, &mut rng) {
                let mut out = EngineOutput::default();
                out.best_move = Some(book_move);
                out.info_lines
                    .push("info string opening book move".to_owned());
                return Ok(out);
            }
        }

        // Honor explicit UCI depth limits first; otherwise fall back to the
        // configured difficulty depth for this engine instance.
        let depth = params.depth.unwrap_or(self.default_depth).max(1);

        let result = iterative_deepening_search(
            game_state,
            &self.move_generator,
            &self.scorer,
            SearchConfig {
                max_depth: depth,
                movetime_ms: params.movetime_ms,
            },
        )
        .map_err(|e| e.to_string())?;

        let mut out = EngineOutput::default();
        out.best_move = result.best_move;
        if out.best_move.is_none() {
            let legal = self
                .move_generator
                .generate_legal_moves(game_state)
                .map_err(|e| e.to_string())?;
            out.best_move = legal.first().map(|m| m.move_description);
        }
        out.info_lines.push(format!(
            "info depth {} score cp {} nodes {}",
            result.reached_depth, result.best_score, result.nodes
        ));
        out.info_lines.push(format!(
            "info string iterative_engine default_depth {}",
            self.default_depth
        ));
        out.info_lines
            .push(format!("info string iterative_engine used_depth {}", depth));
        if let Some(ms) = params.movetime_ms {
            out.info_lines
                .push(format!("info string iterative_engine movetime_ms {}", ms));
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::IterativeEngine;
    use crate::engines::engine_trait::{Engine, GoParams};
    use crate::game_state::game_state::GameState;

    #[test]
    fn iterative_engine_honors_go_depth_override() {
        let game = GameState::new_game();
        let mut engine = IterativeEngine::new(5);
        let params = GoParams {
            depth: Some(1),
            ..GoParams::default()
        };

        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let joined = out.info_lines.join("\n");

        assert!(
            joined.contains("info depth 1"),
            "expected depth-1 search info"
        );
        assert!(
            joined.contains("used_depth 1"),
            "expected used_depth=1 info"
        );
    }
}
