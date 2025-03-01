use std::{collections::LinkedList, fmt::Error};
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

pub fn MoveBoardLocation(x: BoardLocation, d_file: i8, d_rank: i8) -> Option<BoardLocation> {
    let y: BoardLocation = (x.0 + d_file, x.1 + d_rank);
    if y.0 < 0 {
        return None;
    }
    if y.0 > 7 {
        return None;
    }
    if y.1 < 0 {
        return None;
    }
    if y.1 > 7 {
        return None;
    }
    Some(y)
}
#[derive(Copy, Clone)]
pub struct PieceRecord {
    class: Class,
    affiliation: Affiliation,
    location: BoardLocation,
}

#[derive(Default)]
pub struct PieceRegister {
    buffer: [[Option<PieceRecord>; 8]; 8],
}

impl PieceRegister {
    fn at(&mut self, x: BoardLocation) -> &mut Option<PieceRecord> {
        &mut self.buffer[x.0 as usize][x.1 as usize]
    }
    fn get(&self, x: BoardLocation) -> &Option<PieceRecord> {
        &self.buffer[x.0 as usize][x.1 as usize]
    }
    fn add_piece_record(&mut self, x: PieceRecord, y: BoardLocation) -> Result<(), String> {
        let _z = Some(self.at(y));
        if _z.is_some() {
            return Err("Piece already at site".into());
        }
        *self.at(y) = Some(x);
        Ok(())
    }
    fn remove_piece_record(&mut self, y: BoardLocation) -> Option<PieceRecord> {
        let z = *self.get(y);
        *self.at(y) = None;
        z
    }
}
