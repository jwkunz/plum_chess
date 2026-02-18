//! Legal move application logic for transitioning game states.
//!
//! Applies packed move descriptions to `GameState`, updating piece placement,
//! castling rights, en-passant state, clocks, side-to-move, and occupancies.

use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_shared::{piece_on_square_any, ALL_PIECE_KINDS};
use crate::moves::move_descriptions::{
    move_from, move_promotion_piece_code, move_to, pack_move_description, piece_kind_from_code,
    FLAG_CAPTURE, FLAG_CASTLING, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT,
};
use crate::search::zobrist::refresh_game_state_hashes;

pub fn apply_move(game_state: &GameState, move_description: u64) -> Result<GameState, String> {
    let from = move_from(move_description);
    let to = move_to(move_description);
    let from_mask = 1u64 << from;
    let to_mask = 1u64 << to;

    let moving_color = game_state.side_to_move;
    let enemy_color = moving_color.opposite();

    let moved_piece = piece_on_square_any(game_state, from)
        .ok_or_else(|| format!("No piece on from-square {from}"))?
        .1;

    let mut next = game_state.clone();

    // Remove moved piece from origin.
    next.pieces[moving_color.index()][moved_piece.index()] &= !from_mask;

    // Handle captures.
    if (move_description & FLAG_EN_PASSANT) != 0 {
        let capture_sq = if moving_color == Color::Light {
            to.checked_sub(8)
                .ok_or("Invalid en-passant capture square for light")?
        } else {
            to.checked_add(8)
                .ok_or("Invalid en-passant capture square for dark")?
        };
        let capture_mask = 1u64 << capture_sq;
        next.pieces[enemy_color.index()][PieceKind::Pawn.index()] &= !capture_mask;
    } else if (move_description & FLAG_CAPTURE) != 0 {
        clear_enemy_piece_on_square(&mut next, enemy_color, to_mask);
    }

    // Place moved/promoted piece on destination.
    let promotion_piece = piece_kind_from_code(move_promotion_piece_code(move_description));
    if let Some(promo) = promotion_piece {
        next.pieces[moving_color.index()][promo.index()] |= to_mask;
    } else {
        next.pieces[moving_color.index()][moved_piece.index()] |= to_mask;
    }

    // Castling rook move.
    if (move_description & FLAG_CASTLING) != 0 && moved_piece == PieceKind::King {
        match (moving_color, from, to) {
            (Color::Light, 4, 6) => move_rook(&mut next, moving_color, 7, 5),
            (Color::Light, 4, 2) => move_rook(&mut next, moving_color, 0, 3),
            (Color::Dark, 60, 62) => move_rook(&mut next, moving_color, 63, 61),
            (Color::Dark, 60, 58) => move_rook(&mut next, moving_color, 56, 59),
            _ => {}
        }
    }

    // Update castling rights.
    update_castling_rights(&mut next, moving_color, from, to, moved_piece);

    // Update en-passant square.
    next.en_passant_square = if (move_description & FLAG_DOUBLE_PAWN_PUSH) != 0 {
        Some((from + to) / 2)
    } else {
        None
    };

    // Update clocks.
    if moved_piece == PieceKind::Pawn || (move_description & FLAG_CAPTURE) != 0 {
        next.halfmove_clock = 0;
    } else {
        next.halfmove_clock = next.halfmove_clock.saturating_add(1);
    }
    if moving_color == Color::Dark {
        next.fullmove_number = next.fullmove_number.saturating_add(1);
    }

    next.side_to_move = enemy_color;
    next.ply = next.ply.saturating_add(1);

    recalc_occupancy(&mut next);
    refresh_game_state_hashes(&mut next);
    next.repetition_history.push(next.zobrist_key);

    Ok(next)
}

/// Apply a move directly to the provided game state and push an undo record.
///
/// This is a migration helper toward full make/unmake search. It avoids full
/// `GameState` cloning in the caller and records enough state to unmake.
pub fn make_move_in_place(game_state: &mut GameState, move_description: u64) -> Result<(), String> {
    let from = move_from(move_description);
    let to = move_to(move_description);
    let from_mask = 1u64 << from;
    let to_mask = 1u64 << to;

    let moving_color = game_state.side_to_move;
    let enemy_color = moving_color.opposite();

    let moved_piece = piece_on_square_any(game_state, from)
        .ok_or_else(|| format!("No piece on from-square {from}"))?
        .1;

    let captured_piece = if (move_description & FLAG_EN_PASSANT) != 0 {
        Some(PieceKind::Pawn)
    } else if (move_description & FLAG_CAPTURE) != 0 {
        piece_on_square_any(game_state, to).map(|(_, p)| p)
    } else {
        None
    };

    let undo = UndoState {
        mv: move_description as Move,
        moved_piece,
        captured_piece,
        prev_pieces: game_state.pieces,
        prev_occupancy_by_color: game_state.occupancy_by_color,
        prev_occupancy_all: game_state.occupancy_all,
        prev_side_to_move: game_state.side_to_move,
        prev_castling_rights: game_state.castling_rights,
        prev_en_passant_square: game_state.en_passant_square,
        prev_halfmove_clock: game_state.halfmove_clock,
        prev_fullmove_number: game_state.fullmove_number,
        prev_ply: game_state.ply,
        prev_repetition_len: game_state.repetition_history.len(),
        prev_zobrist_key: game_state.zobrist_key,
        prev_pawn_zobrist_key: game_state.pawn_zobrist_key,
    };
    game_state.undo_stack.push(undo);

    // Remove moved piece from origin.
    game_state.pieces[moving_color.index()][moved_piece.index()] &= !from_mask;

    // Handle captures.
    if (move_description & FLAG_EN_PASSANT) != 0 {
        let capture_sq = if moving_color == Color::Light {
            to.checked_sub(8)
                .ok_or("Invalid en-passant capture square for light")?
        } else {
            to.checked_add(8)
                .ok_or("Invalid en-passant capture square for dark")?
        };
        let capture_mask = 1u64 << capture_sq;
        game_state.pieces[enemy_color.index()][PieceKind::Pawn.index()] &= !capture_mask;
    } else if (move_description & FLAG_CAPTURE) != 0 {
        clear_enemy_piece_on_square(game_state, enemy_color, to_mask);
    }

    // Place moved/promoted piece on destination.
    let promotion_piece = piece_kind_from_code(move_promotion_piece_code(move_description));
    if let Some(promo) = promotion_piece {
        game_state.pieces[moving_color.index()][promo.index()] |= to_mask;
    } else {
        game_state.pieces[moving_color.index()][moved_piece.index()] |= to_mask;
    }

    // Castling rook move.
    if (move_description & FLAG_CASTLING) != 0 && moved_piece == PieceKind::King {
        match (moving_color, from, to) {
            (Color::Light, 4, 6) => move_rook(game_state, moving_color, 7, 5),
            (Color::Light, 4, 2) => move_rook(game_state, moving_color, 0, 3),
            (Color::Dark, 60, 62) => move_rook(game_state, moving_color, 63, 61),
            (Color::Dark, 60, 58) => move_rook(game_state, moving_color, 56, 59),
            _ => {}
        }
    }

    // Update castling rights.
    update_castling_rights(game_state, moving_color, from, to, moved_piece);

    // Update en-passant square.
    game_state.en_passant_square = if (move_description & FLAG_DOUBLE_PAWN_PUSH) != 0 {
        Some((from + to) / 2)
    } else {
        None
    };

    // Update clocks.
    if moved_piece == PieceKind::Pawn || (move_description & FLAG_CAPTURE) != 0 {
        game_state.halfmove_clock = 0;
    } else {
        game_state.halfmove_clock = game_state.halfmove_clock.saturating_add(1);
    }
    if moving_color == Color::Dark {
        game_state.fullmove_number = game_state.fullmove_number.saturating_add(1);
    }

    game_state.side_to_move = enemy_color;
    game_state.ply = game_state.ply.saturating_add(1);

    recalc_occupancy(game_state);
    refresh_game_state_hashes(game_state);
    game_state.repetition_history.push(game_state.zobrist_key);

    Ok(())
}

/// Undo the last in-place move applied with `make_move_in_place`.
pub fn unmake_move_in_place(game_state: &mut GameState) -> Result<(), String> {
    let undo = game_state
        .undo_stack
        .pop()
        .ok_or("undo stack is empty; cannot unmake move")?;

    game_state.pieces = undo.prev_pieces;
    game_state.occupancy_by_color = undo.prev_occupancy_by_color;
    game_state.occupancy_all = undo.prev_occupancy_all;
    game_state.side_to_move = undo.prev_side_to_move;
    game_state.castling_rights = undo.prev_castling_rights;
    game_state.en_passant_square = undo.prev_en_passant_square;
    game_state.halfmove_clock = undo.prev_halfmove_clock;
    game_state.fullmove_number = undo.prev_fullmove_number;
    game_state.ply = undo.prev_ply;
    game_state.zobrist_key = undo.prev_zobrist_key;
    game_state.pawn_zobrist_key = undo.prev_pawn_zobrist_key;
    game_state
        .repetition_history
        .truncate(undo.prev_repetition_len);

    Ok(())
}

#[inline]
pub fn build_move(
    from: Square,
    to: Square,
    moved_piece: PieceKind,
    captured_piece: Option<PieceKind>,
    promotion_piece: Option<PieceKind>,
    flags: u64,
) -> u64 {
    pack_move_description(
        from,
        to,
        moved_piece,
        captured_piece,
        promotion_piece,
        flags,
    )
}

fn clear_enemy_piece_on_square(game_state: &mut GameState, enemy_color: Color, square_mask: u64) {
    for piece in ALL_PIECE_KINDS {
        game_state.pieces[enemy_color.index()][piece.index()] &= !square_mask;
    }
}

fn move_rook(game_state: &mut GameState, color: Color, from: Square, to: Square) {
    let from_mask = 1u64 << from;
    let to_mask = 1u64 << to;
    game_state.pieces[color.index()][PieceKind::Rook.index()] &= !from_mask;
    game_state.pieces[color.index()][PieceKind::Rook.index()] |= to_mask;
}

fn update_castling_rights(
    game_state: &mut GameState,
    moving_color: Color,
    from: Square,
    to: Square,
    moved_piece: PieceKind,
) {
    if moved_piece == PieceKind::King {
        if moving_color == Color::Light {
            game_state.castling_rights &= !(CASTLE_LIGHT_KINGSIDE | CASTLE_LIGHT_QUEENSIDE);
        } else {
            game_state.castling_rights &= !(CASTLE_DARK_KINGSIDE | CASTLE_DARK_QUEENSIDE);
        }
    }

    if moved_piece == PieceKind::Rook {
        match from {
            0 => game_state.castling_rights &= !CASTLE_LIGHT_QUEENSIDE,
            7 => game_state.castling_rights &= !CASTLE_LIGHT_KINGSIDE,
            56 => game_state.castling_rights &= !CASTLE_DARK_QUEENSIDE,
            63 => game_state.castling_rights &= !CASTLE_DARK_KINGSIDE,
            _ => {}
        }
    }

    // Capturing rook on original squares also removes rights.
    match to {
        0 => game_state.castling_rights &= !CASTLE_LIGHT_QUEENSIDE,
        7 => game_state.castling_rights &= !CASTLE_LIGHT_KINGSIDE,
        56 => game_state.castling_rights &= !CASTLE_DARK_QUEENSIDE,
        63 => game_state.castling_rights &= !CASTLE_DARK_KINGSIDE,
        _ => {}
    }
}

fn recalc_occupancy(game_state: &mut GameState) {
    game_state.occupancy_by_color[Color::Light.index()] = game_state.pieces[Color::Light.index()]
        .iter()
        .copied()
        .fold(0u64, |acc, bb| acc | bb);
    game_state.occupancy_by_color[Color::Dark.index()] = game_state.pieces[Color::Dark.index()]
        .iter()
        .copied()
        .fold(0u64, |acc, bb| acc | bb);
    game_state.occupancy_all = game_state.occupancy_by_color[Color::Light.index()]
        | game_state.occupancy_by_color[Color::Dark.index()];
}

#[cfg(test)]
mod tests {
    use super::{make_move_in_place, unmake_move_in_place};
    use crate::game_state::game_state::GameState;
    use crate::utils::long_algebraic::long_algebraic_to_move_description;

    #[test]
    fn make_unmake_round_trip_restores_fen_and_hash() {
        let original = GameState::new_game();
        let mut state = original.clone();
        let mv = long_algebraic_to_move_description("e2e4", &state).expect("move parse");
        make_move_in_place(&mut state, mv).expect("make move");
        assert_ne!(state.get_fen(), original.get_fen());
        unmake_move_in_place(&mut state).expect("unmake move");
        assert_eq!(state.get_fen(), original.get_fen());
        assert_eq!(state.zobrist_key, original.zobrist_key);
        assert_eq!(state.pawn_zobrist_key, original.pawn_zobrist_key);
    }
}
