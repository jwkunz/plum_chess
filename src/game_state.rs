use crate::types::*;

pub struct GameState {
    piece_register: PieceRegister,
    can_castle_queen_light: bool,
    can_castle_king_light: bool,
    can_en_passant_light: bool,
    can_castle_queen_dark: bool,
    can_castle_king_dark: bool,
    can_en_passant_dark: bool,
    move_count: u16,
    turn: Affiliation,
}

impl GameState {
    pub fn from_fen(x: String) -> Self {
        GameState {
            piece_register: PieceRegister::default(),
            can_castle_king_dark: true,
            can_castle_king_light: true,
            can_castle_queen_dark: true,
            can_castle_queen_light: true,
            can_en_passant_dark: true,
            can_en_passant_light: true,
            move_count: 0,
            turn: Affiliation::Light,
        }
    }
    pub fn new_game(x: String) -> Self {
        GameState {
            piece_register: PieceRegister::default(),
            can_castle_king_dark: true,
            can_castle_king_light: true,
            can_castle_queen_dark: true,
            can_castle_queen_light: true,
            can_en_passant_dark: true,
            can_en_passant_light: true,
            move_count: 0,
            turn: Affiliation::Light,
        }
    }
    pub fn get_fen(&self) -> String {
        " ".to_string()
    }
}
