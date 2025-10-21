use crate::{board_location::BoardLocation, piece_class::PieceClass};

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
    TryToViewOrEditEmptySquare(BoardLocation),
    InvalidFileOrRank((u8,u8)),
    InvalidFENtoken(char),
    InvalidFEDstringForm(String),
    GeneratingWrongMovementForPieceType(PieceClass),
    FeatureNotImplementedYet,
    InvalidDirectionSelected(u8),
    KingKeyRecordGotCorrupted,
    PieceRegisterDoesNotContainAKing,
    ErrorDuringCheckInspection(String),
}