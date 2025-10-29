//! Scoring utilities for the chess engine.
//!
//! This module centralizes piece valuations, score comparison utilities, and
//! sentinel values to represent winning/losing positions. Scores are modeled
//! as floating point values (Score) to allow fractional heuristics and future
//! weighting adjustments. Several helper functions provide conventional piece
//! values and conversions tailored to a particular side's perspective.
//!
//! Conventions:
//! - Positive scores favor the Light side; negative scores favor the Dark side.
//! - generate_winning_score / generate_losing_score return extreme sentinel
//!   values used to indicate forced win/loss conditions in search/evaluation.

use crate::{piece_class::PieceClass, piece_team::PieceTeam};

/// Numeric representation of an evaluation score.
///
/// A Score represents the engine's evaluation of a position from the perspective
/// where positive values favor the Light side and negative values favor the
/// Dark side. A floating-point type is used to allow fractional/weighted
/// heuristics and to support very large sentinel values for forced outcomes.
pub type Score = f32;

/// Conventional material value for a given PieceClass.
///
/// These are standard approximate piece valuations used by many chess engines
/// to produce a simple material evaluation. Values are returned as a Score and
/// intended to be combined with positional heuristics elsewhere.
///
/// - Pawn:   1.0
/// - Knight: 3.0
/// - Bishop: 3.0
/// - Rook:   5.0
/// - Queen:  9.0
/// - King:   64.0 (a large sentinel-like value; kings are effectively priceless)
pub fn conventional_score(x : &PieceClass) -> Score{
    match x {
        PieceClass::Pawn => 1.0,
        PieceClass::Knight => 3.0,
        PieceClass::Bishop => 3.0,
        PieceClass::Rook => 5.0,
        PieceClass::Queen => 9.0,
        PieceClass::King => 64.0,
    }
}

/// Result of comparing two scores from potentially different sides' perspectives.
///
/// Used to determine which of two candidate evaluations is better when the
/// candidates may be reported relative to different teams.
pub enum ScoreComparison{
    /// The left score is strictly better for the player owning `left_turn`.
    Better,
    /// The two scores are equal (within exact floating equality).
    Equal,
    /// The left score is strictly worse for the player owning `left_turn`.
    Worse
}

/// Compare two scores while accounting for the owning team of each score.
///
/// The function normalizes both scores to the same sign convention (positive
/// means Light is better, negative means Dark is better) and then compares
/// the resulting numeric values.
///
/// # Arguments
/// - `left_score`: numeric value representing the left evaluation.
/// - `left_turn`: team associated with `left_score` (Light/Dark).
/// - `right_score`: numeric value representing the right evaluation.
/// - `right_turn`: team associated with `right_score` (Light/Dark).
///
/// # Returns
/// - `ScoreComparison::Better` if the normalized left score > normalized right score.
/// - `ScoreComparison::Worse` if the normalized left score < normalized right score.
/// - `ScoreComparison::Equal` if they are numerically equal.
pub fn compare_scores(left_score : Score, left_turn : PieceTeam, right_score : Score, right_turn : PieceTeam) -> ScoreComparison{
    let left = match left_turn{
        PieceTeam::Light => left_score,
        PieceTeam::Dark => -left_score,
    };
    let right = match right_turn{
        PieceTeam::Light => right_score,
        PieceTeam::Dark => -right_score,
    };
    if left > right{
        return ScoreComparison::Better;
    }
    if left < right{
        return ScoreComparison::Worse;
    }
    ScoreComparison::Equal
}

/// A very low sentinel score used to represent a decisive loss.
///
/// This constant is chosen large in magnitude so it dominates ordinary heuristic
/// differences and can be used by search algorithms to indicate forced loss.
pub const MIN_SCORE : Score = -1E9;
/// A very high sentinel score used to represent a decisive win.
///
/// This constant is chosen large in magnitude so it dominates ordinary heuristic
/// differences and can be used by search algorithms to indicate forced win.
pub const MAX_SCORE : Score = 1E9;

/// Produce the canonical winning score for the given side.
///
/// Returns MAX_SCORE for Light and MIN_SCORE for Dark so that the returned value
/// is numerically favorable when viewed from the standard sign convention
/// (positive favors Light).
pub fn generate_winning_score(turn : PieceTeam) -> Score{
    match turn {
       PieceTeam::Light => MAX_SCORE,
       PieceTeam::Dark => MIN_SCORE 
    }
}

/// Produce the canonical losing score for the given side.
///
/// This returns the opposite extreme of generate_winning_score for the input
/// team (e.g., if Light's winning score is MAX_SCORE, Light's losing score will
/// be MIN_SCORE). The returned value is numerically unfavorable from the
/// standard sign convention for the provided team.
pub fn generate_losing_score(turn : PieceTeam) -> Score{
    match turn {
       PieceTeam::Light => generate_winning_score(PieceTeam::Dark),
       PieceTeam::Dark => generate_winning_score(PieceTeam::Light)
    }
}