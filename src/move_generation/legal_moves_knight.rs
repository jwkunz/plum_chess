use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_apply::build_move;
use crate::moves::knight_moves::knight_attacks;
use crate::moves::move_descriptions::FLAG_CAPTURE;

pub fn generate_knight_moves(game_state: &GameState, out: &mut Vec<u64>) {
    let side = game_state.side_to_move;
    let own_occ = game_state.occupancy_by_color[side.index()];
    let enemy_occ = game_state.occupancy_by_color[side.opposite().index()];

    let mut knights = game_state.pieces[side.index()][PieceKind::Knight.index()];
    while knights != 0 {
        let from = knights.trailing_zeros() as Square;
        let mut attacks = knight_attacks(from) & !own_occ;

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
                PieceKind::Knight,
                captured,
                None,
                if is_capture { FLAG_CAPTURE } else { 0 },
            ));
            attacks &= attacks - 1;
        }

        knights &= knights - 1;
    }
}

fn enemy_piece_on(game_state: &GameState, square: Square) -> Option<PieceKind> {
    let enemy = game_state.side_to_move.opposite();
    let mask = 1u64 << square;
    for piece in [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ] {
        if (game_state.pieces[enemy.index()][piece.index()] & mask) != 0 {
            return Some(piece);
        }
    }
    None
}
