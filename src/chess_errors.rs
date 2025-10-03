use crate::board_location::BoardLocation;

/// Represents all possible error types that can occur in the chess engine.
/// Used throughout the codebase for error handling and reporting.
#[derive(Debug)]
pub enum ChessErrors {
    FailedTest,
    TriedToMoveOutOfBounds((BoardLocation,i8,i8)),
    InvalidAlgebraicChar(char),
    InvalidAlgebraicString(String),
    CannotRemoveFromEmptyLocation(BoardLocation),
    CannotRemoveKings(BoardLocation),
    TryToViewOrEditEmptySquare(BoardLocation)
}