use std::collections::LinkedList;

use crate::{board_location::BoardLocation, chess_errors::ChessErrors, piece_record::PieceRecord, board_mask::BoardMask};

#[derive(Clone, Debug)]
pub struct PieceRegister {
    pub light_king : PieceRecord,
    pub dark_king : PieceRecord,
    pub light_pieces : LinkedList<PieceRecord>,
    pub dark_pieces : LinkedList<PieceRecord>,
}

impl PieceRegister {
    pub fn new() -> Self{
        PieceRegister { 
            light_king: PieceRecord { class: crate::piece_class::PieceClass::King, location: BoardLocation::from_file_rank(0, 5).unwrap(), team: crate::piece_team::PieceTeam::Light }, 
            dark_king: PieceRecord { class: crate::piece_class::PieceClass::King, location: BoardLocation::from_file_rank(0, 5).unwrap(), team: crate::piece_team::PieceTeam::Dark}, 
            light_pieces: LinkedList::new(), 
            dark_pieces: LinkedList::new()}
    }
    pub fn generate_mask_all_light(&self)->BoardMask{
        let mut result = self.light_king.location.binary_location;
        for i in &self.light_pieces{
            result |= i.location.binary_location;
        }
        result
    }
    pub fn generate_mask_all_dark(&self)->BoardMask{
        let mut result = self.dark_king.location.binary_location;
        for i in &self.dark_pieces{
            result |= i.location.binary_location;
        }
        result
    }  
    pub fn generate_mask_all_pieces(&self)->BoardMask{
        self.generate_mask_all_dark() | self.generate_mask_all_light()
    }  
    pub fn generate_mask_light_king(&self)->BoardMask{
        self.light_king.location.binary_location
    }
    pub fn generate_mask_dark_king(&self)->BoardMask{
        self.dark_king.location.binary_location
    }    
    pub fn view_piece_at_location(&self, x : BoardLocation) -> Result<&PieceRecord, ChessErrors>{
        if self.light_king.location.binary_location == x.binary_location{
            return Ok(&self.light_king)
        }
        if self.dark_king.location.binary_location == x.binary_location{
            return Ok(&self.dark_king)
        }
        for i in &self.light_pieces{
            if i.location.binary_location == x.binary_location{
                return Ok(i)
            }
        }
        for i in &self.dark_pieces{
            if i.location.binary_location == x.binary_location{
                return Ok(i)
            }
        }        
        Err(ChessErrors::TryToViewOrEditEmptySquare(x))
    }
    pub fn edit_piece_at_location(&mut self, x : BoardLocation) -> Result<&mut PieceRecord, ChessErrors>{
        if self.light_king.location.binary_location == x.binary_location{
            return Ok(&mut self.light_king)
        }
        if self.dark_king.location.binary_location == x.binary_location{
            return Ok(&mut self.dark_king)
        }
        for i in &mut self.light_pieces{
            if i.location.binary_location == x.binary_location{
                return Ok(i)
            }
        }
        for i in &mut self.dark_pieces{
            if i.location.binary_location == x.binary_location{
                return Ok(i)
            }
        }        
        Err(ChessErrors::TryToViewOrEditEmptySquare(x))
    }    
    pub fn remove_piece_at_location(&mut self, x : BoardLocation) -> Result<PieceRecord,ChessErrors>{
        if self.light_king.location.binary_location == x.binary_location{
            return Err(ChessErrors::CannotRemoveKings(x))
        }
        if self.dark_king.location.binary_location == x.binary_location{
            return Err(ChessErrors::CannotRemoveKings(x))
        }
        let found_piece : LinkedList<PieceRecord> = self.light_pieces.extract_if(|y| y.location.binary_location == x.binary_location).collect();
        if found_piece.len() > 0{
            return found_piece.front().copied().ok_or(ChessErrors::CannotRemoveFromEmptyLocation(x));
        }
        let found_piece : LinkedList<PieceRecord> = self.dark_pieces.extract_if(|y| y.location.binary_location == x.binary_location).collect();
        if found_piece.len() > 0{
            return found_piece.front().copied().ok_or(ChessErrors::CannotRemoveFromEmptyLocation(x));
        }
        Err(ChessErrors::CannotRemoveFromEmptyLocation(x))
    }    
    pub fn add_piece_record_no_rule_checking(&mut self, x : PieceRecord){
        match x.team {
            crate::piece_team::PieceTeam::Light => {
                if matches!(x.class , crate::piece_class::PieceClass::King){
                    self.light_king = x;
                }else{
                    self.light_pieces.push_back(x);
                }
            },
            crate::piece_team::PieceTeam::Dark =>{
                if matches!(x.class , crate::piece_class::PieceClass::King){
                    self.dark_king = x;
                }else{
                    self.dark_pieces.push_back(x);
                }
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



