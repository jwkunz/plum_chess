use crate::game_state::chess_types::{PieceKind, Square};

const FROM_SHIFT: u64 = 0;
const TO_SHIFT: u64 = 6;
const MOVED_PIECE_SHIFT: u64 = 12;
const CAPTURED_PIECE_SHIFT: u64 = 15;
const PROMOTION_PIECE_SHIFT: u64 = 18;

const SQUARE_MASK: u64 = 0x3F;
const PIECE_MASK: u64 = 0x7;
pub const NO_PIECE_CODE: u64 = 0x7;

pub const FLAG_CAPTURE: u64 = 1u64 << 21;
pub const FLAG_DOUBLE_PAWN_PUSH: u64 = 1u64 << 22;
pub const FLAG_EN_PASSANT: u64 = 1u64 << 23;
pub const FLAG_CASTLING: u64 = 1u64 << 24;

#[inline]
pub fn pack_move_description(
    from: Square,
    to: Square,
    moved_piece: PieceKind,
    captured_piece: Option<PieceKind>,
    promotion_piece: Option<PieceKind>,
    flags: u64,
) -> u64 {
    let mut out = 0u64;
    out |= (from as u64) << FROM_SHIFT;
    out |= (to as u64) << TO_SHIFT;
    out |= piece_kind_to_code(moved_piece) << MOVED_PIECE_SHIFT;
    out |= captured_piece
        .map(piece_kind_to_code)
        .unwrap_or(NO_PIECE_CODE)
        << CAPTURED_PIECE_SHIFT;
    out |= promotion_piece
        .map(piece_kind_to_code)
        .unwrap_or(NO_PIECE_CODE)
        << PROMOTION_PIECE_SHIFT;
    out |= flags;
    out
}

#[inline]
pub fn move_from(move_description: u64) -> Square {
    ((move_description >> FROM_SHIFT) & SQUARE_MASK) as Square
}

#[inline]
pub fn move_to(move_description: u64) -> Square {
    ((move_description >> TO_SHIFT) & SQUARE_MASK) as Square
}

#[inline]
pub fn move_moved_piece_code(move_description: u64) -> u64 {
    (move_description >> MOVED_PIECE_SHIFT) & PIECE_MASK
}

#[inline]
pub fn move_captured_piece_code(move_description: u64) -> u64 {
    (move_description >> CAPTURED_PIECE_SHIFT) & PIECE_MASK
}

#[inline]
pub fn move_promotion_piece_code(move_description: u64) -> u64 {
    (move_description >> PROMOTION_PIECE_SHIFT) & PIECE_MASK
}

#[inline]
pub fn piece_kind_to_code(piece_kind: PieceKind) -> u64 {
    piece_kind.index() as u64
}

#[inline]
pub fn piece_kind_from_code(code: u64) -> Option<PieceKind> {
    match code {
        0 => Some(PieceKind::Pawn),
        1 => Some(PieceKind::Knight),
        2 => Some(PieceKind::Bishop),
        3 => Some(PieceKind::Rook),
        4 => Some(PieceKind::Queen),
        5 => Some(PieceKind::King),
        _ => None,
    }
}

