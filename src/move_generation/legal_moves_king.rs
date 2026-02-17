use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_apply::build_move;
use crate::move_generation::legal_move_checks::is_square_attacked;
use crate::moves::king_moves::king_attacks;
use crate::moves::move_descriptions::{FLAG_CAPTURE, FLAG_CASTLING};

pub fn generate_king_moves(game_state: &GameState, out: &mut Vec<u64>) {
    let side = game_state.side_to_move;
    let own_occ = game_state.occupancy_by_color[side.index()];
    let enemy_occ = game_state.occupancy_by_color[side.opposite().index()];
    let king_bb = game_state.pieces[side.index()][PieceKind::King.index()];
    if king_bb == 0 {
        return;
    }

    let from = king_bb.trailing_zeros() as Square;
    let mut attacks = king_attacks(from) & !own_occ;
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
            PieceKind::King,
            captured,
            None,
            if is_capture { FLAG_CAPTURE } else { 0 },
        ));
        attacks &= attacks - 1;
    }

    generate_castling_moves(game_state, out, from);
}

fn generate_castling_moves(game_state: &GameState, out: &mut Vec<u64>, king_from: Square) {
    let side = game_state.side_to_move;
    let enemy = side.opposite();

    // Cannot castle out of check.
    if is_square_attacked(game_state, king_from, enemy) {
        return;
    }

    match side {
        Color::Light => {
            if king_from == 4 && (game_state.castling_rights & CASTLE_LIGHT_KINGSIDE) != 0 {
                let empty = (1u64 << 5) | (1u64 << 6);
                if (game_state.occupancy_all & empty) == 0
                    && !is_square_attacked(game_state, 5, enemy)
                    && !is_square_attacked(game_state, 6, enemy)
                {
                    out.push(build_move(4, 6, PieceKind::King, None, None, FLAG_CASTLING));
                }
            }
            if king_from == 4 && (game_state.castling_rights & CASTLE_LIGHT_QUEENSIDE) != 0 {
                let empty = (1u64 << 1) | (1u64 << 2) | (1u64 << 3);
                if (game_state.occupancy_all & empty) == 0
                    && !is_square_attacked(game_state, 3, enemy)
                    && !is_square_attacked(game_state, 2, enemy)
                {
                    out.push(build_move(4, 2, PieceKind::King, None, None, FLAG_CASTLING));
                }
            }
        }
        Color::Dark => {
            if king_from == 60 && (game_state.castling_rights & CASTLE_DARK_KINGSIDE) != 0 {
                let empty = (1u64 << 61) | (1u64 << 62);
                if (game_state.occupancy_all & empty) == 0
                    && !is_square_attacked(game_state, 61, enemy)
                    && !is_square_attacked(game_state, 62, enemy)
                {
                    out.push(build_move(60, 62, PieceKind::King, None, None, FLAG_CASTLING));
                }
            }
            if king_from == 60 && (game_state.castling_rights & CASTLE_DARK_QUEENSIDE) != 0 {
                let empty = (1u64 << 57) | (1u64 << 58) | (1u64 << 59);
                if (game_state.occupancy_all & empty) == 0
                    && !is_square_attacked(game_state, 59, enemy)
                    && !is_square_attacked(game_state, 58, enemy)
                {
                    out.push(build_move(60, 58, PieceKind::King, None, None, FLAG_CASTLING));
                }
            }
        }
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
