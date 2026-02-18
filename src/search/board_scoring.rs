//! Pluggable board evaluation interfaces and baseline implementations.
//!
//! Search remains modular by delegating static position scoring to this trait,
//! allowing alternate heuristics to be swapped without altering search code.

use crate::game_state::{chess_types::*, game_state::GameState};
use crate::moves::bishop_moves::bishop_attacks;
use crate::moves::king_moves::king_attacks;
use crate::moves::knight_moves::knight_attacks;
use crate::moves::pawn_moves::pawn_attacks;
use crate::moves::queen_moves::queen_attacks;
use crate::moves::rook_moves::rook_attacks;

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

#[derive(Debug, Clone, Copy, Default)]
pub struct StandardScorer;

impl StandardScorer {
    const MOBILITY_WEIGHT: i32 = 2;

    fn positional_term(game_state: &GameState) -> i32 {
        let mut score = 0i32;
        for color in [Color::Light, Color::Dark] {
            let sign = if color == Color::Light { 1 } else { -1 };
            for piece in [
                PieceKind::Pawn,
                PieceKind::Knight,
                PieceKind::Bishop,
                PieceKind::Rook,
                PieceKind::Queen,
                PieceKind::King,
            ] {
                let mut bb = game_state.pieces[color.index()][piece.index()];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as u8;
                    score += sign * piece_square_bonus(piece, color, sq);
                    bb &= bb - 1;
                }
            }
        }
        score
    }

    fn mobility_term(game_state: &GameState) -> i32 {
        let mut white = 0i32;
        let mut black = 0i32;
        let occ = game_state.occupancy_all;
        let own_w = game_state.occupancy_by_color[Color::Light.index()];
        let own_b = game_state.occupancy_by_color[Color::Dark.index()];

        // White mobility
        white += mobility_for_color(game_state, Color::Light, occ, own_w);
        // Black mobility
        black += mobility_for_color(game_state, Color::Dark, occ, own_b);

        (white - black) * Self::MOBILITY_WEIGHT
    }
}

impl BoardScorer for StandardScorer {
    fn score(&self, game_state: &GameState) -> i32 {
        let material = MaterialScorer::material_balance_white_minus_black(game_state);
        let positional = Self::positional_term(game_state);
        let mobility = Self::mobility_term(game_state);
        let white_minus_black = material + positional + mobility;
        match game_state.side_to_move {
            Color::Light => white_minus_black,
            Color::Dark => -white_minus_black,
        }
    }
}

fn mobility_for_color(game_state: &GameState, color: Color, occ: u64, own_occ: u64) -> i32 {
    let mut m = 0i32;
    let idx = color.index();

    let mut pawns = game_state.pieces[idx][PieceKind::Pawn.index()];
    while pawns != 0 {
        let sq = pawns.trailing_zeros() as u8;
        m += (pawn_attacks(color, sq) & !own_occ).count_ones() as i32;
        pawns &= pawns - 1;
    }

    let mut knights = game_state.pieces[idx][PieceKind::Knight.index()];
    while knights != 0 {
        let sq = knights.trailing_zeros() as u8;
        m += (knight_attacks(sq) & !own_occ).count_ones() as i32;
        knights &= knights - 1;
    }

    let mut bishops = game_state.pieces[idx][PieceKind::Bishop.index()];
    while bishops != 0 {
        let sq = bishops.trailing_zeros() as u8;
        m += (bishop_attacks(sq, occ) & !own_occ).count_ones() as i32;
        bishops &= bishops - 1;
    }

    let mut rooks = game_state.pieces[idx][PieceKind::Rook.index()];
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as u8;
        m += (rook_attacks(sq, occ) & !own_occ).count_ones() as i32;
        rooks &= rooks - 1;
    }

    let mut queens = game_state.pieces[idx][PieceKind::Queen.index()];
    while queens != 0 {
        let sq = queens.trailing_zeros() as u8;
        m += (queen_attacks(sq, occ) & !own_occ).count_ones() as i32;
        queens &= queens - 1;
    }

    let mut kings = game_state.pieces[idx][PieceKind::King.index()];
    while kings != 0 {
        let sq = kings.trailing_zeros() as u8;
        m += (king_attacks(sq) & !own_occ).count_ones() as i32;
        kings &= kings - 1;
    }

    m
}

fn piece_square_bonus(piece: PieceKind, color: Color, sq: u8) -> i32 {
    let rank = (sq / 8) as i32;
    let file = (sq % 8) as i32;
    let r = if color == Color::Light {
        rank
    } else {
        7 - rank
    };
    let dist_center = (file - 3).abs() + (r - 3).abs();
    let center_bonus = 4 - dist_center;

    match piece {
        PieceKind::Pawn => r * 8 - (file - 3).abs() * 2,
        PieceKind::Knight => center_bonus * 6,
        PieceKind::Bishop => center_bonus * 4 + r,
        PieceKind::Rook => r * 2,
        PieceKind::Queen => center_bonus * 2,
        PieceKind::King => {
            // Mild opening preference for castled/edge king.
            if r <= 1 {
                8 - (file - 4).abs() * 2
            } else {
                -center_bonus * 4
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardScorer, MaterialScorer, StandardScorer};
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

    #[test]
    fn standard_scorer_rewards_central_knight() {
        let center =
            GameState::from_fen("4k3/8/8/3N4/8/8/8/4K3 w - - 0 1").expect("FEN should parse");
        let rim = GameState::from_fen("4k3/8/8/8/8/8/N7/4K3 w - - 0 1").expect("FEN should parse");
        let scorer = StandardScorer;
        assert!(
            scorer.score(&center) > scorer.score(&rim),
            "central knight should score better"
        );
    }
}
