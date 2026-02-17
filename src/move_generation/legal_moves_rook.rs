//! Legal rook move generation.
//!
//! Emits packed move descriptions for rook moves from the current side to
//! move, including captures and special rules where applicable.

use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_apply::build_move;
use crate::move_generation::legal_move_shared::enemy_piece_on;
use crate::moves::move_descriptions::FLAG_CAPTURE;
use crate::moves::rook_moves::rook_attacks;

pub fn generate_rook_moves(game_state: &GameState, out: &mut Vec<u64>) {
    let side = game_state.side_to_move;
    let own_occ = game_state.occupancy_by_color[side.index()];
    let enemy_occ = game_state.occupancy_by_color[side.opposite().index()];

    let mut rooks = game_state.pieces[side.index()][PieceKind::Rook.index()];
    while rooks != 0 {
        let from = rooks.trailing_zeros() as Square;
        let mut attacks = rook_attacks(from, game_state.occupancy_all) & !own_occ;

        while attacks != 0 {
            let to = attacks.trailing_zeros() as Square;
            let to_mask = 1u64 << to;
            let is_capture = (to_mask & enemy_occ) != 0;
            let captured = if is_capture {
                enemy_piece_on(game_state, to)
            } else {
                None
            };
            out.push(build_move(
                from,
                to,
                PieceKind::Rook,
                captured,
                None,
                if is_capture { FLAG_CAPTURE } else { 0 },
            ));
            attacks &= attacks - 1;
        }

        rooks &= rooks - 1;
    }
}
