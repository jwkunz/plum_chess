use crate::{board_mask::BoardMask, piece_register::PieceRegister};

#[derive(Debug,Clone)]
pub struct CollisionMasks{
    pub light_mask : BoardMask,
    pub dark_mask : BoardMask
}

/// Creates a CollisionMasks populated from the given PieceRegister.
///
/// Constructs a CollisionMasks by computing the occupancy masks for all light
/// and dark pieces using the provided `PieceRegister`. The function borrows the
/// register immutably and does not take ownership.
///
/// # Parameters
/// - `piece_register`: Reference to the `PieceRegister` used to generate the
///   light and dark occupancy masks.
///
/// # Returns
/// A `CollisionMasks` with `light_mask` set to
/// `piece_register.generate_mask_all_light()` and `dark_mask` set to
/// `piece_register.generate_mask_all_dark()`.
impl CollisionMasks{
    pub fn from(piece_register : &PieceRegister) ->Self{
        CollisionMasks{
            light_mask : piece_register.generate_mask_all_light(),
            dark_mask : piece_register.generate_mask_all_dark(),
        }
    }
}