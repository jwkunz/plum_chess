use crate::errors::Errors;
use crate::types::*;

pub struct GameState {
    piece_register: PieceRegister,
    can_castle_queen_light: bool,
    can_castle_king_light: bool,
    can_castle_queen_dark: bool,
    can_castle_king_dark: bool,
    en_passant_location: Option<BoardLocation>,
    half_move_clock: u16,
    full_move_count: u16,
    turn: Affiliation,
}

enum FENflag {
    EndLine,
    LightMove,
    DarkMove,
    Space,
    CastleKingLight,
    CastleKingDark,
    CastleQueenLight,
    CastleQueenDark,
    EnPassantFile,
    EnPassantRank,
    EnPassantNone,
}

impl GameState {
    pub fn from_fen(x: String) -> Result<Self, Errors> {
        let mut piece_register = PieceRegister::default();
        let mut can_castle_king_dark: bool = false;
        let mut can_castle_king_light: bool = false;
        let mut can_castle_queen_dark: bool = false;
        let mut can_castle_queen_light: bool = false;
        let mut en_passant_location = None;
        let mut half_move_clock: u16 = 0;
        let mut full_move_count: u16 = 0;
        let mut turn: Affiliation = Affiliation::Light;

        let mut fields = x.split_ascii_whitespace();

        if let Some(position_field) = fields.next() {
            let mut location: BoardLocation = (0, 7);
            for i in position_field.chars() {
                match i {
                    'r' => {
                        let x = PieceRecord {
                            class: Class::Rook,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'n' => {
                        let x = PieceRecord {
                            class: Class::Knight,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'b' => {
                        let x = PieceRecord {
                            class: Class::Bishop,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'q' => {
                        let x = PieceRecord {
                            class: Class::Queen,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'k' => {
                        let x = PieceRecord {
                            class: Class::King,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'p' => {
                        let x = PieceRecord {
                            class: Class::Pawn,
                            affiliation: Affiliation::Dark,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'P' => {
                        let x = PieceRecord {
                            class: Class::Pawn,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'R' => {
                        let x = PieceRecord {
                            class: Class::Rook,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'N' => {
                        let x = PieceRecord {
                            class: Class::Knight,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'B' => {
                        let x = PieceRecord {
                            class: Class::Bishop,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'Q' => {
                        let x = PieceRecord {
                            class: Class::Queen,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'K' => {
                        let x = PieceRecord {
                            class: Class::King,
                            affiliation: Affiliation::Light,
                        };
                        piece_register.add_piece_record(x, location)?;
                        location = match MoveBoardLocation(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    '/' => {
                        location = match MoveBoardLocation(location, 0, -1) {
                            Ok(new_location) => (0, new_location.1),
                            Err(_) => location,
                        }
                    }
                    '1'..='8' => {
                        let x = i.to_digit(10).expect("This char should parse") as i8;
                        if (x == 8) && (location.0 == 0) {
                            continue;
                        }
                        location = match MoveBoardLocation(location, x, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => return Err(Errors::InvalidFENstring),
                        }
                    }
                    _ => return Err(Errors::InvalidFENstring),
                };
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        if let Some(turn_field) = fields.next() {
            if let Some(c) = turn_field.chars().next() {
                turn = match c {
                    'w' => Affiliation::Light,
                    'b' => Affiliation::Dark,
                    _ => return Err(Errors::InvalidFENstring),
                }
            } else {
                return Err(Errors::InvalidFENstring);
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        if let Some(castle_field) = fields.next() {
            for c in castle_field.chars() {
                match c {
                    'k' => can_castle_king_dark = true,
                    'q' => can_castle_queen_dark = true,
                    'K' => can_castle_king_light = true,
                    'Q' => can_castle_queen_light = true,
                    '-' => (),
                    _ => return Err(Errors::InvalidFENstring),
                }
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        if let Some(en_passant_field) = fields.next() {
            let mut iter = en_passant_field.chars();
            let mut file = Some(0);
            let mut rank = Some(0);
            if let Some(c) = iter.next() {
                file = match c {
                    'a' => Some(0),
                    'b' => Some(1),
                    'c' => Some(2),
                    'e' => Some(3),
                    'e' => Some(4),
                    'f' => Some(5),
                    'g' => Some(6),
                    'h' => Some(7),
                    '-' => None,
                    _ => return Err(Errors::InvalidFENstring),
                }
            }
            if let Some(c) = iter.next() {
                rank = match c {
                    '1' => Some(0),
                    '2' => Some(1),
                    '3' => Some(2),
                    '4' => Some(3),
                    '5' => Some(4),
                    '6' => Some(5),
                    '7' => Some(6),
                    '8' => Some(7),
                    '-' => None,
                    _ => return Err(Errors::InvalidFENstring),
                }
            }
            if (file.is_some()) && (rank.is_some()) {
                en_passant_location = Some((file.unwrap(), rank.unwrap()));
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        if let Some(half_move_clock_str) = fields.next() {
            if let Ok(c) = half_move_clock_str.parse::<u16>() {
                half_move_clock = c;
            } else {
                return Err(Errors::InvalidFENstring);
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        if let Some(full_move_field_str) = fields.next() {
            if let Ok(c) = full_move_field_str.parse::<u16>() {
                full_move_count = c;
            } else {
                return Err(Errors::InvalidFENstring);
            }
        } else {
            return Err(Errors::InvalidFENstring);
        }

        Ok(GameState {
            piece_register,
            can_castle_king_dark,
            can_castle_king_light,
            can_castle_queen_dark,
            can_castle_queen_light,
            en_passant_location,
            half_move_clock,
            full_move_count,
            turn,
        })
    }
    pub fn new_game() -> Self {
        let new_game = String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        GameState::from_fen(new_game).expect("New game string must have been corrupted")
    }
    pub fn get_fen(&self) -> String {
        " ".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_new_game() {
        let dut = GameState::new_game();
    }
}
