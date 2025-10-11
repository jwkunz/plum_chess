use crate::{board_mask::BoardMask, piece_register::PieceRegister};

#[derive(Debug,Clone)]
pub struct CollisionMasks{
    pub light_mask : BoardMask,
    pub dark_mask : BoardMask
}

impl CollisionMasks{
    pub fn from(piece_register : &PieceRegister) ->Self{
        CollisionMasks{
            light_mask : piece_register.generate_mask_all_light(),
            dark_mask : piece_register.generate_mask_all_dark(),
        }
    }
}