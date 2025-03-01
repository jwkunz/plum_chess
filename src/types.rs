use crate::errors::*;
#[derive(Copy, Clone)]
pub enum Class {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}
#[derive(Copy, Clone)]
pub enum Affiliation {
    Dark,
    Light,
}

pub type BoardLocation = (i8, i8);

pub fn move_board_location(
    x: BoardLocation,
    d_file: i8,
    d_rank: i8,
) -> Result<BoardLocation, Errors> {
    let y: BoardLocation = (x.0 + d_file, x.1 + d_rank);
    if (y.0 < 0) | (y.0 > 7) | (y.1 < 0) | (y.1 > 7) {
        Err(Errors::OutOfBounds)
    } else {
        Ok(y)
    }
}
#[derive(Copy, Clone)]
pub struct PieceRecord {
    pub class: Class,
    pub affiliation: Affiliation,
}

#[derive(Default, Clone)]
pub struct PieceRegister {
    buffer: [[Option<PieceRecord>; 8]; 8],
}

impl PieceRegister {
    pub fn at(&mut self, x: BoardLocation) -> &mut Option<PieceRecord> {
        &mut self.buffer[x.0 as usize][x.1 as usize]
    }
    pub fn view(&self, x: BoardLocation) -> &Option<PieceRecord> {
        &self.buffer[x.0 as usize][x.1 as usize]
    }
    pub fn add_piece_record(&mut self, x: PieceRecord, y: BoardLocation) -> Result<(), Errors> {
        let _z = self.at(y);
        if _z.is_some() {
            return Err(Errors::BoardLocationOccupied);
        }
        *self.at(y) = Some(x);
        Ok(())
    }
    pub fn remove_piece_record(&mut self, y: BoardLocation) -> Option<PieceRecord> {
        let z = *self.view(y);
        *self.at(y) = None;
        z
    }
}
