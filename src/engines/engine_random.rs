//! Difficulty-1 random-move engine.
//!
//! Selects uniformly from legal moves and is primarily used for diagnostics,
//! integration testing, and low-strength gameplay.

use rand::prelude::IndexedRandom;

use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::LegalMoveGenerator;
use crate::move_generation::move_generator::MoveGenerator;

pub struct RandomEngine {
    move_generator: LegalMoveGenerator,
}

impl RandomEngine {
    pub fn new() -> Self {
        Self {
            move_generator: LegalMoveGenerator,
        }
    }
}

impl Default for RandomEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for RandomEngine {
    fn name(&self) -> &str {
        "PlumChess Random"
    }

    fn author(&self) -> &str {
        "jwkunz+codex"
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        let legal_moves = self
            .move_generator
            .generate_legal_moves(game_state)
            .map_err(|e| e.to_string())?;

        let mut out = EngineOutput::default();
        out.info_lines.push(format!(
            "info string random_engine legal_moves {}",
            legal_moves.len()
        ));

        if let Some(depth) = params.depth {
            out.info_lines.push(format!(
                "info string random_engine requested_depth {}",
                depth
            ));
        }

        if legal_moves.is_empty() {
            out.best_move = None;
            return Ok(out);
        }

        let mut rng = rand::rng();
        let picked = legal_moves
            .as_slice()
            .choose(&mut rng)
            .ok_or("failed to choose a random move")?;

        out.best_move = Some(picked.move_description);
        Ok(out)
    }
}
