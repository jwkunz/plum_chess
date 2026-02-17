/// Core game state representation for a high-performance bitboard engine.
/// This module intentionally contains structural placeholders to be filled in
/// as move generation, hashing, and search are implemented.

pub use crate::game_state::game_state::GameState;
pub use crate::game_state::undo_state::UndoState;

/// Side to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Light,
    Dark,
}

impl Color {
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Color::Light => 0,
            Color::Dark => 1,
        }
    }

    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Color::Light => Color::Dark,
            Color::Dark => Color::Light,
        }
    }
}

/// Piece kind (color is represented separately for cache-friendly layouts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            PieceKind::Pawn => 0,
            PieceKind::Knight => 1,
            PieceKind::Bishop => 2,
            PieceKind::Rook => 3,
            PieceKind::Queen => 4,
            PieceKind::King => 5,
        }
    }
}

/// Packed move type placeholder. The exact bit layout is defined later.
pub type Move = u32;



/// Compact castling rights bitmask placeholder.
pub const CASTLE_LIGHT_KINGSIDE: CastlingRights = 1 << 0;
pub const CASTLE_LIGHT_QUEENSIDE: CastlingRights = 1 << 1;
pub const CASTLE_DARK_KINGSIDE: CastlingRights = 1 << 2;
pub const CASTLE_DARK_QUEENSIDE: CastlingRights = 1 << 3;
pub type CastlingRights = u8;

/// Board square index placeholder (`0..=63`).
pub type Square = u8;
