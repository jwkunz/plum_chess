//! Errors used throughout the chess engine.
//!
//! This module defines the canonical error type returned by game logic,
//! parsing utilities, move-generation and other core subsystems. The enum
//! `ChessErrors` is used as the single error type across the crate to simplify
//! propagation and matching. Each variant carries contextual information where
//! appropriate to aid diagnostics and user-facing error messages.
//!
//! Usage guidelines:
//! - Functions in the engine should return `Result<..., ChessErrors>` for
//!   recoverable or expected failure modes (invalid input, illegal moves, etc).
//! - Callers should match on `ChessErrors` to present friendly messages or to
//!   implement domain-specific recovery (for example falling back to a default
//!   move when parsing fails).
//! - Variants that represent internal corruption or unimplemented features
//!   (e.g. `KingKeyRecordGotCorrupted`, `FeatureNotImplementedYet`) indicate
//!   bugs or incomplete features and are not intended to be recovered from by
//!   normal library users.

use crate::{board_location::BoardLocation, piece_class::PieceClass};

/// Unified error type for the chess engine.
///
/// Each variant corresponds to a specific, identifiable failure mode that can
/// occur while manipulating the game state, parsing algebraic notation, or
/// running internal algorithms. Variants include contextual payloads where
/// useful (for example `BoardLocation` or offending character/token) so that
/// callers can log or display precise diagnostics.
///
/// When matching on `ChessErrors`:
/// - Treat `FailedTest`, `FeatureNotImplementedYet`, and corruption-style
///   variants as internal errors or TODOs (likely not recoverable in normal
///   operation).
/// - Treat parsing and input-related variants (`InvalidAlgebraicString`,
///   `InvalidFileOrRank`, etc.) as recoverable and suitable for presenting to
///   end users.
/// - Treat game-state violation variants (illegal moves, missing king, etc.)
///   as domain-level errors requiring specific handling in game logic / UIs.
#[derive(Debug)]
pub enum ChessErrors {
    /// Generic failure used in tests or as a catch-all when no more specific
    /// variant applies.
    ///
    /// Intended primarily for unit tests and quick-fail code paths.
    FailedTest,

    /// Attempted to move a piece from `BoardLocation` by the delta `(d_file,d_rank)`
    /// which would place it off the board.
    ///
    /// Payload: (origin_location, d_file, d_rank)
    TriedToMoveOutOfBounds((BoardLocation, i8, i8)),

    /// A single character used during algebraic parsing was invalid.
    ///
    /// Payload: the offending character (for example a file outside 'a'..'h' or
    /// a rank outside '1'..'8').
    InvalidAlgebraicChar(char),

    /// An algebraic string (multi-character) failed to parse.
    ///
    /// Payload: the original string that could not be interpreted as a move or
    /// square.
    InvalidAlgebraicString(String),

    /// Attempted to remove a piece from an empty square.
    ///
    /// Payload: the location that was expected to contain a piece.
    CannotRemoveFromEmptyLocation(BoardLocation),

    /// Attempted to remove a king piece when such an operation is not permitted.
    ///
    /// Payload: the location of the king that the caller tried to remove.
    CannotRemoveKings(BoardLocation),

    /// Attempted to view or edit a square that is empty (no piece present).
    ///
    /// Payload: the empty square's location.
    TryToViewOrEditEmptySquare(BoardLocation),

    /// Invalid file or rank indices were provided (outside 0..=7).
    ///
    /// Payload: (file_index, rank_index) zero-based.
    InvalidFileOrRank((u8, u8)),

    /// Found an unexpected token while parsing a FEN-like or FED string.
    ///
    /// Payload: the offending character/token.
    InvalidFENtoken(char),

    /// FEN-like string had malformed structure (not matching expected form).
    ///
    /// Payload: the original offending string for diagnostics.
    InvalidFEDstringForm(String),

    /// A piece-specific movement generator was invoked for the wrong piece type.
    ///
    /// Payload: the PieceClass that was used (for example generating pawn moves
    /// with a rook generator).
    GeneratingWrongMovementForPieceType(PieceClass),

    /// A feature is referenced that is not implemented yet in the engine.
    ///
    /// Use this variant as a placeholder for functionality that should be added.
    FeatureNotImplementedYet,

    /// An invalid direction index was selected for an operation that expects
    /// a small set of direction identifiers (for example a slider direction).
    ///
    /// Payload: the invalid index value.
    InvalidDirectionSelected(u8),

    /// Internal corruption detected in the king-key record (used for hashing or
    /// internal records). This indicates a bug or memory/state corruption.
    KingKeyRecordGotCorrupted,

    /// The piece register (board snapshot) does not contain a king for one side.
    ///
    /// This represents a corrupted or invalid game state; callers should treat
    /// this as a fatal logic error in game construction or maintenance.
    PieceRegisterDoesNotContainAKing,

    /// An error occurred while performing check inspection logic; payload
    /// contains implementation-specific diagnostic output.
    ErrorDuringCheckInspection(String),

    /// No legal moves are available for the side to move (used to indicate
    /// checkmate or stalemate conditions at higher-level logic).
    NoLegalMoves,
}