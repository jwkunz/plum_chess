use std::collections::HashMap;

use crate::{board_location::{BinaryLocation, BoardLocation}, board_mask::BoardMask, chess_errors::ChessErrors, piece_record::PieceRecord, piece_team::PieceTeam};

#[derive(Clone, Debug)]
pub struct PieceRegister {
    light_king_key : Option<BinaryLocation>,
    dark_king_key : Option<BinaryLocation>,
    pub light_pieces : HashMap<u64,PieceRecord>,
    pub dark_pieces : HashMap<u64,PieceRecord>,
}

impl PieceRegister {
    pub fn new() -> Self{
        let light_pieces = HashMap::<u64,PieceRecord>::with_capacity(18);
        let dark_pieces = HashMap::<u64,PieceRecord>::with_capacity(18);
        let light_king_key=None;
        let dark_king_key=None;
        PieceRegister { 
            light_king_key, 
            dark_king_key, 
            light_pieces, 
            dark_pieces}
    }
    pub fn generate_mask_all_light(&self)->BoardMask{
        let mut result = 0;
        for (_,value) in &self.light_pieces{
            result |= value.location.binary_location;
        }
        result
    }
    pub fn generate_mask_all_dark(&self)->BoardMask{
        let mut result = 0;
        for (_,value) in &self.dark_pieces{
            result |= value.location.binary_location;
        }
        result
    }  
    pub fn generate_mask_all_pieces(&self)->BoardMask{
        self.generate_mask_all_dark() | self.generate_mask_all_light()
    }  
    pub fn generate_mask_light_king(&self)->Result<BoardMask,ChessErrors>{
        Ok(self.view_king(PieceTeam::Light)?.location.binary_location)
    }
    pub fn generate_mask_dark_king(&self)->Result<BoardMask,ChessErrors>{
        Ok(self.view_king(PieceTeam::Dark)?.location.binary_location)
    }    
    pub fn view_piece_at_location(&self, x : BoardLocation) -> Result<&PieceRecord, ChessErrors>{
        if let Some(piece) = self.light_pieces.get(&x.binary_location){
            return Ok(piece);
        }else if let Some(piece) = self.dark_pieces.get(&x.binary_location){
            return Ok(piece);
        }
        Err(ChessErrors::TryToViewOrEditEmptySquare(x))
    }
    pub fn view_king(&self, x : PieceTeam) -> Result<&PieceRecord, ChessErrors>{
        match x{
            PieceTeam::Light =>{
            Ok(&self.light_pieces[&self.light_king_key.ok_or_else(|| ChessErrors::PieceRegisterDoesNotContainAKing)?])
            }
            PieceTeam::Dark =>{
            Ok(&self.dark_pieces[&self.dark_king_key.ok_or_else(|| ChessErrors::PieceRegisterDoesNotContainAKing)?])
            }
        }
    }   
    pub fn edit_piece_at_location(&mut self, x : BoardLocation) -> Result<&mut PieceRecord, ChessErrors>{
        if let Some(piece) = self.light_pieces.get_mut(&x.binary_location){
            return Ok(piece);
        }else if let Some(piece) = self.dark_pieces.get_mut(&x.binary_location){
            return Ok(piece);
        }
        Err(ChessErrors::TryToViewOrEditEmptySquare(x))
    }    
    /// Moves a piece from the start to destination.
    /// If a piece is already at the destination it erases that piece 
    /// The option piece in the result tuple is the captured piece if present
    pub fn move_piece_to_location_with_overwrite(&mut self, start : BoardLocation, destination : BoardLocation) -> Result<Option<PieceRecord>, ChessErrors>{
        let mut start_piece = self.remove_piece_at_location(start)?;
        start_piece.location = destination;
        let captured = match start_piece.team {
            crate::piece_team::PieceTeam::Light => {
                if matches!(start_piece.class, crate::piece_class::PieceClass::King){
                    self.light_king_key = Some(destination.binary_location);
                }
                self.light_pieces.insert(destination.binary_location, start_piece)
            },
            crate::piece_team::PieceTeam::Dark => {
                if matches!(start_piece.class, crate::piece_class::PieceClass::King){
                    self.dark_king_key = Some(destination.binary_location);
                }     
                self.dark_pieces.insert(destination.binary_location, start_piece)
            },
        };
        Ok(captured)
    }     
    pub fn remove_piece_at_location(&mut self, x : BoardLocation) -> Result<PieceRecord,ChessErrors>{
        if self.light_king_key == Some(x.binary_location){
            self.light_king_key = None;
        }
        if self.dark_king_key == Some(x.binary_location){
            self.dark_king_key = None;
        }
        if let Some(piece) = self.light_pieces.remove(&x.binary_location){
            return Ok(piece);
        }else if let Some(piece) = self.dark_pieces.remove(&x.binary_location){
            return Ok(piece);
        }
        Err(ChessErrors::CannotRemoveFromEmptyLocation(x))
    }    
    pub fn add_piece_record_no_rule_checking(&mut self, x : PieceRecord){
        match x.team {
            crate::piece_team::PieceTeam::Light => {
                if matches!(x.class , crate::piece_class::PieceClass::King){
                    self.light_king_key = Some(x.location.binary_location);
                }
                self.light_pieces.insert(x.location.binary_location,x);
            },
            crate::piece_team::PieceTeam::Dark =>{
                if matches!(x.class , crate::piece_class::PieceClass::King){
                    self.dark_king_key = Some(x.location.binary_location);
                }
                self.dark_pieces.insert(x.location.binary_location,x);
            }
        }
    }
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn add_remove_pieces() -> Result<(),ChessErrors>{
        let mut dut = PieceRegister::new();
        dut.add_piece_record_no_rule_checking(PieceRecord { class: crate::piece_class::PieceClass::Pawn, location: BoardLocation::from_file_rank(0, 1).unwrap(), team: crate::piece_team::PieceTeam::Light });
        dut.add_piece_record_no_rule_checking(PieceRecord { class: crate::piece_class::PieceClass::Pawn, location: BoardLocation::from_file_rank(0, 2).unwrap(), team: crate::piece_team::PieceTeam::Light });
        let _ = dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 1).unwrap())?;
        let _ = dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 2).unwrap())?;
        if dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 1).unwrap()).is_err(){
            return Ok(())
        }
        Err(ChessErrors::FailedTest)
    }
}



