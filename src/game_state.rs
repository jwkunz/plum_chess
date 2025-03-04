use crate::errors::Errors;
use crate::types::*;

#[derive(Clone)]
pub struct GameState {
    pub piece_register: PieceRegister,
    pub can_castle_queen_light: bool,
    pub can_castle_king_light: bool,
    pub can_castle_queen_dark: bool,
    pub can_castle_king_dark: bool,
    pub en_passant_location: Option<BoardLocation>,
    pub half_move_clock: u16,
    pub full_move_count: u16,
    pub turn: Affiliation,
}

impl GameState {
    pub fn from_fen(x: &str) -> Result<Self, Errors> {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
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
                        location = match move_board_location(location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    '/' => {
                        location = match move_board_location(location, 0, -1) {
                            Ok(new_location) => (0, new_location.1),
                            Err(_) => location,
                        }
                    }
                    '1'..='8' => {
                        let x = i.to_digit(10).expect("This char should parse") as i8;
                        location = match move_board_location(location, x, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => (7, location.1),
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
                    'd' => Some(3),
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
        GameState::from_fen(&new_game).expect("New game string must have been corrupted")
    }
    pub fn get_fen(&self) -> String {
        let mut result = String::new();
        for i in (0..8).rev() {
            let mut space_count: u8 = 0;
            for j in 0..8 {
                if let Some(x) = self.piece_register.view((j, i)) {
                    if space_count > 0 {
                        result.push(space_count.to_string().chars().next().unwrap());
                    }
                    let c: char = match x.affiliation {
                        Affiliation::Light => match x.class {
                            Class::Bishop => 'B',
                            Class::King => 'K',
                            Class::Knight => 'N',
                            Class::Pawn => 'P',
                            Class::Queen => 'Q',
                            Class::Rook => 'R',
                        },
                        Affiliation::Dark => match x.class {
                            Class::Bishop => 'b',
                            Class::King => 'k',
                            Class::Knight => 'n',
                            Class::Pawn => 'p',
                            Class::Queen => 'q',
                            Class::Rook => 'r',
                        },
                    };
                    result.push(c);
                    space_count = 0;
                } else {
                    space_count += 1;
                }
            }
            if space_count > 0 {
                result.push(space_count.to_string().chars().next().unwrap());
            }
            if i > 0 {
                result.push('/');
            }
        }

        result.push(' ');
        match self.turn {
            Affiliation::Dark => result.push('b'),
            Affiliation::Light => result.push('w'),
        };
        result.push(' ');

        if self.can_castle_king_light {
            result.push('K');
        }
        if self.can_castle_queen_light {
            result.push('Q');
        }
        if self.can_castle_king_dark {
            result.push('k');
        }
        if self.can_castle_queen_dark {
            result.push('q');
        }
        if !self.can_castle_king_dark
            | !self.can_castle_queen_dark
            | !self.can_castle_king_light
            | !self.can_castle_queen_light
        {
            result.push('-');
        }
        result.push(' ');

        if self.en_passant_location.is_some() {
            match self.en_passant_location.unwrap().0 {
                0 => result.push('a'),
                1 => result.push('b'),
                2 => result.push('c'),
                3 => result.push('d'),
                4 => result.push('e'),
                5 => result.push('f'),
                6 => result.push('g'),
                7 => result.push('h'),
                _ => panic!("position file got corrupted"),
            }
            match self.en_passant_location.unwrap().1 {
                0 => result.push('1'),
                1 => result.push('2'),
                2 => result.push('3'),
                3 => result.push('4'),
                4 => result.push('5'),
                5 => result.push('6'),
                6 => result.push('7'),
                7 => result.push('8'),
                _ => panic!("position rank got corrupted"),
            }
        } else {
            result.push('-');
        }
        result.push(' ');

        result.push_str(&self.half_move_clock.to_string());
        result.push(' ');

        result.push_str(&self.full_move_count.to_string());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_new_game() {
        let dut = GameState::new_game();
        let new_game_string =
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
        assert_eq!(dut.get_fen(), new_game_string);

        let game_string_0 =
            "1r4k1/7p/3p1bp1/p1pP4/P1P1prP1/1N2R2P/1P1N1PK1/8 b - - 3 31".to_string();
        let dut_0 = GameState::from_fen(&game_string_0).expect("Should parse this string");
        let r_0 = dut_0.get_fen();
        assert_eq!(r_0, game_string_0);

        let game_string_1 =
            "r1bq1rk1/ppp2ppp/2n5/2bp4/4n3/1P2PNP1/PBP2PBP/RN1Q1RK1 b - - 2 9".to_string();
        let dut_1 = GameState::from_fen(&game_string_1).expect("Should parse this string");
        let r_1 = dut_1.get_fen();
        assert_eq!(r_1, game_string_1);

        let game_string_2 = "8/bpp1k2p/p2pP1p1/P5q1/1P5N/8/6PP/5Q1K b - - 0 35".to_string();
        let dut_2 = GameState::from_fen(&game_string_2).expect("Should parse this string");
        let r_2 = dut_2.get_fen();
        assert_eq!(r_2, game_string_2);
    }
}
