use crate::move_description::MoveDescription;
use crate::piece_record::PieceRecord;

/// Descriptions of Check
#[derive(Clone, Copy, Debug)]
pub enum TypesOfCheck {
    /// Check (King piece)
    SingleCheck(PieceRecord),
    /// Check (King piece, other threatening piece)
    DoubleCheck(PieceRecord, PieceRecord),
    /// Check (King piece,pinned_piece)
    Pin(PieceRecord, PieceRecord),
}

/// Contains a move description +
/// If there is a check event
#[derive(Clone, Debug)]
pub struct CheckedMoveDescription {
    pub description: MoveDescription,
    pub check_status: Option<TypesOfCheck>,
}