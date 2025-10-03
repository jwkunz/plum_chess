use crate::piece_class::PieceClass;

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