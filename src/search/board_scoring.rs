//! Pluggable board evaluation interfaces and baseline implementations.
//!
//! Search remains modular by delegating static position scoring to this trait,
//! allowing alternate heuristics to be swapped without altering search code.

use crate::game_state::{chess_types::*, game_state::GameState};

pub const MATE_SCORE: i32 = 30000;
pub trait BoardScorer: Send + Sync {
    /// Score from the perspective of the side to move.
    fn score(&self, game_state: &GameState) -> i32;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MaterialScorer;

impl MaterialScorer {
    #[inline]
    pub const fn piece_value(piece: PieceKind) -> i32 {
        match piece {
            PieceKind::Pawn => 100,
            PieceKind::Knight => 320,
            PieceKind::Bishop => 330,
            PieceKind::Rook => 500,
            PieceKind::Queen => 900,
            PieceKind::King => 5000,
        }
    }

    #[inline]
    fn material_balance_white_minus_black(game_state: &GameState) -> i32 {
        let mut score = 0i32;

        for piece in [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ] {
            let value = Self::piece_value(piece);
            let white_count =
                game_state.pieces[Color::Light.index()][piece.index()].count_ones() as i32;
            let black_count =
                game_state.pieces[Color::Dark.index()][piece.index()].count_ones() as i32;
            score += (white_count - black_count) * value;
        }

        score
    }
}

impl BoardScorer for MaterialScorer {
    fn score(&self, game_state: &GameState) -> i32 {
        let white_minus_black = Self::material_balance_white_minus_black(game_state);
        match game_state.side_to_move {
            Color::Light => white_minus_black,
            Color::Dark => -white_minus_black,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardScorer, MaterialScorer};
    use crate::game_state::game_state::GameState;

    #[test]
    fn material_scorer_reflects_side_to_move_perspective() {
        let white_to_move =
            GameState::from_fen("4k3/8/8/8/8/8/8/4KQ2 w - - 0 1").expect("FEN should parse");
        let black_to_move =
            GameState::from_fen("4k3/8/8/8/8/8/8/4KQ2 b - - 0 1").expect("FEN should parse");

        let scorer = MaterialScorer;
        assert_eq!(scorer.score(&white_to_move), 900);
        assert_eq!(scorer.score(&black_to_move), -900);
    }
}
