use crate::board_location::BoardLocation;

/// The special stuff for castling rights and en passant
#[derive(Clone,Debug)]
pub struct SpecialMoveFlags{
    /// Whether light (white) can castle queenside.
    pub can_castle_queen_light: bool,
    /// Whether light (white) can castle kingside.
    pub can_castle_king_light: bool,
    /// Whether dark (black) can castle queenside.
    pub can_castle_queen_dark: bool,
    /// Whether dark (black) can castle kingside.
    pub can_castle_king_dark: bool,
    /// The en passant flag (space behind victim piece)
    pub en_passant_location: Option<BoardLocation>,
}