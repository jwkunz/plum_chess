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

