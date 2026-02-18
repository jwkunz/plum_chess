//! Undo delta structures for reversible move application.
//!
//! The undo payload stores a compact pre-move delta so search can restore
//! previous positions efficiently without snapshotting full piece arrays.

use crate::game_state::chess_types::*;

/// Single undo record for `make_move` / `unmake_move`.
#[derive(Debug, Clone)]
pub struct UndoState {
    pub mv: Move,
    pub moved_piece: PieceKind,
    pub captured_piece: Option<PieceKind>,

    pub prev_side_to_move: Color,
    pub prev_castling_rights: CastlingRights,
    pub prev_en_passant_square: Option<Square>,
    pub prev_halfmove_clock: u16,
    pub prev_fullmove_number: u16,
    pub prev_ply: u16,
    pub prev_repetition_len: usize,

    pub prev_zobrist_key: u64,
    pub prev_pawn_zobrist_key: u64,
}
