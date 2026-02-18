//! Legal move application logic for transitioning game states.
//!
//! Applies packed move descriptions to `GameState`, updating piece placement,
//! castling rights, en-passant state, clocks, side-to-move, and occupancies.

use crate::game_state::{chess_types::*, game_state::GameState};
use crate::move_generation::legal_move_shared::piece_on_square_any;
use crate::moves::move_descriptions::{
    move_from, move_promotion_piece_code, move_to, pack_move_description, piece_kind_from_code,
    FLAG_CAPTURE, FLAG_CASTLING, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT,
};
use crate::search::zobrist::{
    castling_key, en_passant_file_key, piece_square_key, side_to_move_key,
};

pub fn apply_move(game_state: &GameState, move_description: u64) -> Result<GameState, String> {
    let mut next = game_state.clone();
    make_move_in_place(&mut next, move_description)?;
    let _ = next.undo_stack.pop();
    Ok(next)
}

/// Apply a move directly to the provided game state and push an undo record.
///
/// This is a migration helper toward full make/unmake search. It avoids full
/// `GameState` cloning in the caller and records enough state to unmake.
pub fn make_move_in_place(game_state: &mut GameState, move_description: u64) -> Result<(), String> {
    let from = move_from(move_description);
    let to = move_to(move_description);

    let moving_color = game_state.side_to_move;
    let enemy_color = moving_color.opposite();
    let prev_castling = game_state.castling_rights;
    let prev_en_passant = game_state.en_passant_square;

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
    remove_piece(game_state, moving_color, moved_piece, from);

    // Handle captures.
    if (move_description & FLAG_EN_PASSANT) != 0 {
        let capture_sq = if moving_color == Color::Light {
            to.checked_sub(8)
                .ok_or("Invalid en-passant capture square for light")?
        } else {
            to.checked_add(8)
                .ok_or("Invalid en-passant capture square for dark")?
        };
        remove_piece(game_state, enemy_color, PieceKind::Pawn, capture_sq);
    } else if (move_description & FLAG_CAPTURE) != 0 {
        if let Some(captured) = captured_piece {
            remove_piece(game_state, enemy_color, captured, to);
        }
    }

    // Place moved/promoted piece on destination.
    let promotion_piece = piece_kind_from_code(move_promotion_piece_code(move_description));
    if let Some(promo) = promotion_piece {
        add_piece(game_state, moving_color, promo, to);
    } else {
        add_piece(game_state, moving_color, moved_piece, to);
    }

    // Castling rook move.
    if (move_description & FLAG_CASTLING) != 0 && moved_piece == PieceKind::King {
        match (moving_color, from, to) {
            (Color::Light, 4, 6) => move_rook(game_state, moving_color, 7, 5)?,
            (Color::Light, 4, 2) => move_rook(game_state, moving_color, 0, 3)?,
            (Color::Dark, 60, 62) => move_rook(game_state, moving_color, 63, 61)?,
            (Color::Dark, 60, 58) => move_rook(game_state, moving_color, 56, 59)?,
            _ => {}
        }
    }

    // Update castling rights.
    update_castling_rights(game_state, moving_color, from, to, moved_piece);
    if prev_castling != game_state.castling_rights {
        game_state.zobrist_key ^= castling_key(prev_castling);
        game_state.zobrist_key ^= castling_key(game_state.castling_rights);
    }

    // Update en-passant square.
    if let Some(prev_ep) = prev_en_passant {
        game_state.zobrist_key ^= en_passant_file_key(prev_ep % 8);
    }
    game_state.en_passant_square = if (move_description & FLAG_DOUBLE_PAWN_PUSH) != 0 {
        Some((from + to) / 2)
    } else {
        None
    };
    if let Some(new_ep) = game_state.en_passant_square {
        game_state.zobrist_key ^= en_passant_file_key(new_ep % 8);
    }

    // Update clocks.
    if moved_piece == PieceKind::Pawn || (move_description & FLAG_CAPTURE) != 0 {
        game_state.halfmove_clock = 0;
    } else {
        game_state.halfmove_clock = game_state.halfmove_clock.saturating_add(1);
    }
    if moving_color == Color::Dark {
        game_state.fullmove_number = game_state.fullmove_number.saturating_add(1);
    }

    game_state.zobrist_key ^= side_to_move_key();
    game_state.side_to_move = enemy_color;
    game_state.ply = game_state.ply.saturating_add(1);

    game_state.occupancy_all = game_state.occupancy_by_color[Color::Light.index()]
        | game_state.occupancy_by_color[Color::Dark.index()];
    game_state.repetition_history.push(game_state.zobrist_key);

    debug_assert_eq!(
        game_state.zobrist_key,
        crate::search::zobrist::compute_zobrist_key(game_state)
    );
    debug_assert_eq!(
        game_state.pawn_zobrist_key,
        crate::search::zobrist::compute_pawn_zobrist_key(game_state)
    );

    Ok(())
}

/// Undo the last in-place move applied with `make_move_in_place`.
pub fn unmake_move_in_place(game_state: &mut GameState) -> Result<(), String> {
    let undo = game_state
        .undo_stack
        .pop()
        .ok_or("undo stack is empty; cannot unmake move")?;

    let moving_color = undo.prev_side_to_move;
    let enemy_color = moving_color.opposite();
    let mv = undo.mv as u64;
    let from = move_from(mv);
    let to = move_to(mv);

    // Restore side-to-move before piece restoration so any derived logic
    // aligns with the pre-move perspective.
    game_state.side_to_move = moving_color;

    // Remove moved/promoted piece from destination and restore mover on origin.
    let promotion_piece = piece_kind_from_code(move_promotion_piece_code(mv));
    if let Some(promo) = promotion_piece {
        remove_piece(game_state, moving_color, promo, to);
    } else {
        remove_piece(game_state, moving_color, undo.moved_piece, to);
    }
    add_piece(game_state, moving_color, undo.moved_piece, from);

    // Undo castling rook move.
    if (mv & FLAG_CASTLING) != 0 && undo.moved_piece == PieceKind::King {
        match (moving_color, from, to) {
            (Color::Light, 4, 6) => move_rook(game_state, moving_color, 5, 7)?,
            (Color::Light, 4, 2) => move_rook(game_state, moving_color, 3, 0)?,
            (Color::Dark, 60, 62) => move_rook(game_state, moving_color, 61, 63)?,
            (Color::Dark, 60, 58) => move_rook(game_state, moving_color, 59, 56)?,
            _ => {}
        }
    }

    // Restore captured piece, if any.
    if (mv & FLAG_EN_PASSANT) != 0 {
        let capture_sq = if moving_color == Color::Light {
            to.checked_sub(8)
                .ok_or("Invalid en-passant capture square for light during unmake")?
        } else {
            to.checked_add(8)
                .ok_or("Invalid en-passant capture square for dark during unmake")?
        };
        add_piece(game_state, enemy_color, PieceKind::Pawn, capture_sq);
    } else if let Some(captured_piece) = undo.captured_piece {
        add_piece(game_state, enemy_color, captured_piece, to);
    }

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
    game_state.occupancy_all = game_state.occupancy_by_color[Color::Light.index()]
        | game_state.occupancy_by_color[Color::Dark.index()];

    debug_assert_eq!(
        game_state.zobrist_key,
        crate::search::zobrist::compute_zobrist_key(game_state)
    );
    debug_assert_eq!(
        game_state.pawn_zobrist_key,
        crate::search::zobrist::compute_pawn_zobrist_key(game_state)
    );

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

#[inline]
fn remove_piece(game_state: &mut GameState, color: Color, piece: PieceKind, square: Square) {
    let mask = 1u64 << square;
    game_state.pieces[color.index()][piece.index()] &= !mask;
    game_state.occupancy_by_color[color.index()] &= !mask;
    game_state.zobrist_key ^= piece_square_key(color, piece, square);
    if matches!(piece, PieceKind::Pawn | PieceKind::King) {
        game_state.pawn_zobrist_key ^= piece_square_key(color, piece, square);
    }
}

#[inline]
fn add_piece(game_state: &mut GameState, color: Color, piece: PieceKind, square: Square) {
    let mask = 1u64 << square;
    game_state.pieces[color.index()][piece.index()] |= mask;
    game_state.occupancy_by_color[color.index()] |= mask;
    game_state.zobrist_key ^= piece_square_key(color, piece, square);
    if matches!(piece, PieceKind::Pawn | PieceKind::King) {
        game_state.pawn_zobrist_key ^= piece_square_key(color, piece, square);
    }
}

fn move_rook(
    game_state: &mut GameState,
    color: Color,
    from: Square,
    to: Square,
) -> Result<(), String> {
    remove_piece(game_state, color, PieceKind::Rook, from);
    add_piece(game_state, color, PieceKind::Rook, to);
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{make_move_in_place, unmake_move_in_place};
    use crate::game_state::game_state::GameState;
    use crate::search::zobrist::{compute_pawn_zobrist_key, compute_zobrist_key};
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

    #[test]
    fn make_move_incremental_hash_matches_recompute() {
        let mut state = GameState::new_game();
        let mv = long_algebraic_to_move_description("e2e4", &state).expect("move parse");
        make_move_in_place(&mut state, mv).expect("make move");
        assert_eq!(state.zobrist_key, compute_zobrist_key(&state));
        assert_eq!(state.pawn_zobrist_key, compute_pawn_zobrist_key(&state));
    }
}
