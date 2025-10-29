use crate::{piece_class::PieceClass, piece_team::PieceTeam};

pub type Score = f32;

/// Conventional score for each piece
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

pub const MIN_SCORE : Score = -1E9; 
pub const MAX_SCORE : Score = 1E9; 
pub enum ScoreComparison{
    Better,
    Equal,
    Worse
}

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

pub fn generate_winning_score(turn : PieceTeam) -> Score{
    match turn {
       PieceTeam::Light => 1000.0,
       PieceTeam::Dark => -1000.0 
    }
}

pub fn generate_losing_score(turn : PieceTeam) -> Score{
    match turn {
       PieceTeam::Light => generate_winning_score(PieceTeam::Dark),
       PieceTeam::Dark => generate_winning_score(PieceTeam::Light)
    }
}