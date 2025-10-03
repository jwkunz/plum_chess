use crate::{board_location::BoardLocation, piece_class::PieceClass, piece_team::PieceTeam};

/// Represents a chess piece with its class and team.
/// Used to store information about a piece on the board.
#[derive(Copy, Clone, Debug)]
pub struct PieceRecord {
    /// The class (type) of the piece (e.g., pawn, knight).
    pub class: PieceClass,
    /// Piece location
    pub location: BoardLocation,
    /// Piece team
    pub team: PieceTeam
}

