use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_apply::build_move;
use crate::move_generation::legal_move_shared::enemy_piece_on;
use crate::moves::move_descriptions::{FLAG_CAPTURE, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT};

pub fn generate_pawn_moves(game_state: &GameState, out: &mut Vec<u64>) {
    let side = game_state.side_to_move;
    let our_pawns = game_state.pieces[side.index()][PieceKind::Pawn.index()];
    let our_occ = game_state.occupancy_by_color[side.index()];
    let enemy_occ = game_state.occupancy_by_color[side.opposite().index()];
    let empty = !game_state.occupancy_all;

    let mut pawns = our_pawns;
    while pawns != 0 {
        let from = pawns.trailing_zeros() as Square;
        let file = from % 8;
        let rank = from / 8;

        let one_step = if side == Color::Light {
            from.checked_add(8)
        } else {
            from.checked_sub(8)
        };

        if let Some(to) = one_step {
            let to_mask = 1u64 << to;
            if (to_mask & empty) != 0 {
                let promotion_rank = if side == Color::Light { 7 } else { 0 };
                if to / 8 == promotion_rank {
                    for promo in [PieceKind::Knight, PieceKind::Bishop, PieceKind::Rook, PieceKind::Queen] {
                        out.push(build_move(from, to, PieceKind::Pawn, None, Some(promo), 0));
                    }
                } else {
                    out.push(build_move(from, to, PieceKind::Pawn, None, None, 0));

                    let start_rank = if side == Color::Light { 1 } else { 6 };
                    if rank == start_rank {
                        let two_step = if side == Color::Light { from + 16 } else { from - 16 };
                        let two_mask = 1u64 << two_step;
                        if (two_mask & empty) != 0 {
                            out.push(build_move(
                                from,
                                two_step,
                                PieceKind::Pawn,
                                None,
                                None,
                                FLAG_DOUBLE_PAWN_PUSH,
                            ));
                        }
                    }
                }
            }
        }

        // captures and en-passant
        for file_delta in [-1i8, 1i8] {
            let new_file = file as i8 + file_delta;
            if !(0..=7).contains(&new_file) {
                continue;
            }

            let to_opt = if side == Color::Light {
                from.checked_add((8 + file_delta) as u8)
            } else {
                from.checked_sub((8 - file_delta) as u8)
            };
            let Some(to) = to_opt else { continue; };
            let to_mask = 1u64 << to;

            if (to_mask & enemy_occ) != 0 {
                let captured_piece = enemy_piece_on(game_state, to);
                let promotion_rank = if side == Color::Light { 7 } else { 0 };
                if to / 8 == promotion_rank {
                    for promo in [PieceKind::Knight, PieceKind::Bishop, PieceKind::Rook, PieceKind::Queen] {
                        out.push(build_move(
                            from,
                            to,
                            PieceKind::Pawn,
                            captured_piece,
                            Some(promo),
                            FLAG_CAPTURE,
                        ));
                    }
                } else {
                    out.push(build_move(
                        from,
                        to,
                        PieceKind::Pawn,
                        captured_piece,
                        None,
                        FLAG_CAPTURE,
                    ));
                }
            } else if game_state.en_passant_square == Some(to) {
                out.push(build_move(
                    from,
                    to,
                    PieceKind::Pawn,
                    Some(PieceKind::Pawn),
                    None,
                    FLAG_CAPTURE | FLAG_EN_PASSANT,
                ));
            }
        }

        pawns &= pawns - 1;
    }

    // Silence unused warning for now if occupancy variable changes later.
    let _ = our_occ;
}
