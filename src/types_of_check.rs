use crate::piece_record::PieceRecord;

/// Descriptions of Check
#[derive(Clone, Copy, Debug)]
pub enum TypesOfCheck {
    // Unspecified check is used when we know there is a check but not the details
    /// Check (King piece, checking piece)
    UnclassifiedCheck(PieceRecord, PieceRecord),
    /// Check (King piece, checking piece)
    SingleCheck(PieceRecord, PieceRecord),
    /// Check (King piece, checking piece)
    DiscoveryCheck(PieceRecord, PieceRecord),    
    /// Check (King piece, checking piece 1, checking piece 2)
    DoubleCheck(PieceRecord, PieceRecord, PieceRecord),
    /// Checkmate
    Checkmate(PieceRecord, PieceRecord),      
}