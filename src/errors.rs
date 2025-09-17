use crate::{board_location::BoardLocation, chess_move::ChessMove};

/// Represents all possible error types that can occur in the chess engine.
/// Used throughout the codebase for error handling and reporting.
#[derive(Debug)]
pub enum Errors {
    /// Indicates an attempted access outside the bounds of the chess board.
    OutOfBounds,
    /// A generic runtime error occurred.
    RuntimeError,
    /// A chess rule was violated (e.g., illegal move, check, etc.).
    GameRuleError,
    /// Attempted to place or move a piece to a square that is already occupied.
    BoardLocationOccupied,
    /// The provided FEN string is invalid or could not be parsed.
    InvalidFENstring,
    /// The provided algebraic notation is invalid or could not be parsed.
    InvalidAlgebraic,
    /// The starting square for a move is invalid (e.g., wrong piece, wrong turn).
    InvalidMoveStartCondition,
    /// Attempted to move a piece that does not exist at the specified location.
    TryingToMoveNonExistantPiece((BoardLocation,String)),
}