use crate::{
    chess_errors::ChessErrors, collision_masks::CollisionMasks, generate_moves_level_1::ListOfRawMoves, generate_moves_level_2::*, piece_record::PieceRecord
};

/*
Level 3 movement generations needs this information:

#[derive(Debug,Clone)]
pub struct GenerateLevel3args{
    pub pice: PieceRecord,
    pub masks: CollisionMasks
}
*/


/// The results from level 3 generation are level 2 generations created from a PieceRecord
#[derive(Debug,Clone)]
pub struct GenerateLevel3Result{
    pub no_collisions : ListOfRawMoves,
    pub captures : ListOfRawMoves,
}
impl GenerateLevel3Result{
    /// Makes a new object
    pub fn new() -> Self{
        GenerateLevel3Result { no_collisions: ListOfRawMoves::new(), captures: ListOfRawMoves::new()}
    }
    
    // Move fields
    pub fn from_level_2(x : GenerateLevel2Result) ->Self{
        GenerateLevel3Result{
            no_collisions: x.no_collisions,
            captures: x.captures
        }
    }
    
    // Unpack and pick moves given the type of piece
    pub fn from(piece : &PieceRecord, masks : &CollisionMasks) -> Result<Self,ChessErrors>{
        Ok(GenerateLevel3Result::from_level_2(
            match piece.class {
                crate::piece_class::PieceClass::Pawn => generate_pawn_moves_level_2(piece.location, masks, piece.team)?,
                crate::piece_class::PieceClass::Bishop => generate_bishop_moves_level_2(piece.location, masks, piece.team)?,
                crate::piece_class::PieceClass::Knight => generate_knight_moves_level_2(piece.location, masks, piece.team)?,
                crate::piece_class::PieceClass::Rook => generate_rook_moves_level_2(piece.location, masks, piece.team)?,
                crate::piece_class::PieceClass::Queen => generate_queen_moves_level_2(piece.location, masks, piece.team)?,
                crate::piece_class::PieceClass::King => generate_king_moves_level_2(piece.location, masks, piece.team)?,
            }
        ))
    }

    /// Counts the number of moves found
    pub fn len(&self) -> usize{
        self.captures.len() + self.no_collisions.len()
    }
}
