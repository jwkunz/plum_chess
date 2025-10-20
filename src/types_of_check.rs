use crate::piece_record::PieceRecord;

/// Descriptions of Check
#[derive(Clone, Copy, Debug)]
pub enum TypesOfCheck {
    /// Check (King piece, checking piece)
    SingleCheck(PieceRecord, PieceRecord),
    /// Check (King piece, checking piece)
    DiscoveryCheck(PieceRecord, PieceRecord),    
    /// Check (King piece, checking piece 1, checking piece 2)
    DoubleCheck(PieceRecord, PieceRecord, PieceRecord),
    /// Checkmate
    Checkmate(PieceRecord, PieceRecord),      
}