//! Undo snapshot structures for reversible move application.
//!
//! The undo payload preserves pre-move state fragments so search can restore
//! previous positions efficiently when traversing game trees.

use crate::game_state::chess_types::*;

/// Single undo record for `make_move` / `unmake_move`.
#[derive(Debug, Clone)]
pub struct UndoState {
    pub mv: Move,
    pub moved_piece: PieceKind,
    pub captured_piece: Option<PieceKind>,

    pub prev_pieces: [[u64; 6]; 2],
    pub prev_occupancy_by_color: [u64; 2],
    pub prev_occupancy_all: u64,

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
