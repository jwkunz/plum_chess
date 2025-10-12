use crate::move_description::MoveDescription;
use crate::types_of_check::TypesOfCheck;

/// Contains a move description +
/// If there is a check event
#[derive(Clone, Debug)]
pub struct CheckedMoveDescription {
    pub description: MoveDescription,
    pub check_status: Option<TypesOfCheck>,
}