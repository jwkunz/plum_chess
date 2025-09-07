use std::fmt::Debug;

#[derive(Copy, Clone, Debug)]
pub enum PieceClass {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PieceTeam {
    Dark,
    Light,
}


#[derive(Copy, Clone, Debug)]
pub struct PieceRecord {
    pub class: PieceClass,
    pub team: PieceTeam,
}

