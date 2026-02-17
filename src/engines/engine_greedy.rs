use rand::prelude::IndexedRandom;

use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::chess_types::PieceKind;
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::LegalMoveGenerator;
use crate::move_generation::move_generator::MoveGenerator;
use crate::moves::move_descriptions::{
    move_captured_piece_code, piece_kind_from_code, NO_PIECE_CODE,
};

pub struct GreedyEngine {
    move_generator: LegalMoveGenerator,
}

impl GreedyEngine {
    pub fn new() -> Self {
        Self {
            move_generator: LegalMoveGenerator,
        }
    }

    #[inline]
    fn piece_value(piece: PieceKind) -> i32 {
        match piece {
            PieceKind::Pawn => 100,
            PieceKind::Knight => 320,
            PieceKind::Bishop => 330,
            PieceKind::Rook => 500,
            PieceKind::Queen => 900,
            PieceKind::King => 20000,
        }
    }
}

impl Default for GreedyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GreedyEngine {
    fn name(&self) -> &str {
        "PlumChess Greedy"
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
            "info string greedy_engine legal_moves {}",
            legal_moves.len()
        ));

        if let Some(depth) = params.depth {
            out.info_lines
                .push(format!("info string greedy_engine requested_depth {}", depth));
        }

        if legal_moves.is_empty() {
            return Ok(out);
        }

        let mut best_value = i32::MIN;
        let mut best_moves = Vec::new();

        for mv in &legal_moves {
            let capture_code = move_captured_piece_code(mv.move_description);
            let capture_value = if capture_code == NO_PIECE_CODE {
                0
            } else {
                let piece = piece_kind_from_code(capture_code)
                    .ok_or_else(|| format!("invalid captured piece code: {capture_code}"))?;
                Self::piece_value(piece)
            };

            if capture_value > best_value {
                best_value = capture_value;
                best_moves.clear();
                best_moves.push(mv.move_description);
            } else if capture_value == best_value {
                best_moves.push(mv.move_description);
            }
        }

        let mut rng = rand::rng();
        let picked = best_moves
            .as_slice()
            .choose(&mut rng)
            .ok_or("failed to choose greedy best move")?;

        out.info_lines.push(format!(
            "info string greedy_engine capture_score {}",
            best_value
        ));
        out.best_move = Some(*picked);
        Ok(out)
    }
}
