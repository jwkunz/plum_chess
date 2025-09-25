use std::fmt::Debug;

/// Represents the type (class) of a chess piece.
/// Used to distinguish between pawns, knights, bishops, rooks, queens, and kings.
#[derive(Copy, Clone, Debug)]
pub enum PieceClass {
    /// A pawn piece.
    Pawn,
    /// A knight piece.
    Knight,
    /// A bishop piece.
    Bishop,
    /// A rook piece.
    Rook,
    /// A queen piece.
    Queen,
    /// A king piece.
    King,
}

/// Conventional score for each piece
pub fn conventional_score(x : &PieceClass) -> u8{
    match x {
        PieceClass::Pawn => 1,
        PieceClass::Knight => 3,
        PieceClass::Bishop => 3,
        PieceClass::Rook => 5,
        PieceClass::Queen => 9,
        PieceClass::King => 64,
    }
}

/// Represents the team (color) of a chess piece.
/// Used to distinguish between dark (black) and light (white) pieces.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PieceTeam {
    /// The dark (black) side.
    Dark,
    /// The light (white) side.
    Light,
}

/// Represents a chess piece with its class and team.
/// Used to store information about a piece on the board.
#[derive(Copy, Clone, Debug)]
pub struct PieceRecord {
    /// The class (type) of the piece (e.g., pawn, knight).
    pub class: PieceClass,
    /// The team (color) of the piece (dark or light).
    pub team: PieceTeam,
}

