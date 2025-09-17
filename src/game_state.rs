use crate::board_location::{move_board_location, BoardLocation};
use crate::errors::Errors;
use crate::piece_register::PieceRegister;
use crate::piece_types::*;

/// Represents the complete state of a chess game at a given moment.
/// Includes piece positions, castling rights, en passant target, move clocks, and turn.
#[derive(Clone)]
pub struct GameState {
    /// The register containing all pieces and their locations.
    pub piece_register: PieceRegister,
    /// Whether light (white) can castle queenside.
    pub can_castle_queen_light: bool,
    /// Whether light (white) can castle kingside.
    pub can_castle_king_light: bool,
    /// Whether dark (black) can castle queenside.
    pub can_castle_queen_dark: bool,
    /// Whether dark (black) can castle kingside.
    pub can_castle_king_dark: bool,
    /// The en passant target square, if any.
    pub en_passant_location: Option<BoardLocation>,
    /// The half-move clock (for the 50-move rule).
    pub half_move_clock: u16,
    /// The full-move count (increments after black's move).
    pub full_move_count: u16,
    /// The team whose turn it is to move.
    pub turn: PieceTeam,
}

impl GameState {
    /// Creates a `GameState` from a FEN string.
    ///
    /// # Arguments
    /// * `x` - A string slice that holds the FEN string.
    ///
    /// # Returns
    /// * `Result<Self, Errors>` - Returns a `GameState` if the FEN string is valid, otherwise returns an error.
    pub fn from_fen(x: &str) -> Result<Self, Errors> {
        let mut piece_register = PieceRegister::default();
        let mut can_castle_king_dark: bool = false;
        let mut can_castle_king_light: bool = false;
        let mut can_castle_queen_dark: bool = false;
        let mut can_castle_queen_light: bool = false;
        let mut en_passant_location = None;
        let half_move_clock: u16;
        let full_move_count: u16;
        let turn;

        let mut fields = x.split_ascii_whitespace();

        if let Some(position_field) = fields.next() {
            let mut location: BoardLocation = (0, 7);
            for i in position_field.chars() {
                match i {
                    'r' => {
                        let x = PieceRecord {
                            class: PieceClass::Rook,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'n' => {
                        let x = PieceRecord {
                            class: PieceClass::Knight,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'b' => {
                        let x = PieceRecord {
                            class: PieceClass::Bishop,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'q' => {
                        let x = PieceRecord {
                            class: PieceClass::Queen,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'k' => {
                        let x = PieceRecord {
                            class: PieceClass::King,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'p' => {
                        let x = PieceRecord {
                            class: PieceClass::Pawn,
                            team: PieceTeam::Dark,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'P' => {
                        let x = PieceRecord {
                            class: PieceClass::Pawn,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'R' => {
                        let x = PieceRecord {
                            class: PieceClass::Rook,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'N' => {
                        let x = PieceRecord {
                            class: PieceClass::Knight,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'B' => {
                        let x = PieceRecord {
                            class: PieceClass::Bishop,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'Q' => {
                        let x = PieceRecord {
                            class: PieceClass::Queen,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    'K' => {
                        let x = PieceRecord {
                            class: PieceClass::King,
                            team: PieceTeam::Light,
                        };
                        piece_register.add_piece_record(x, &location)?;
                        location = match move_board_location(&location, 1, 0) {
                            Ok(new_location) => new_location,
                            Err(_) => location,
                        }
                    }
                    '/' => {
                        location = match move_board_location(&location, 0, -1) {
                            Ok(new_location) => (0, new_location.1),
                            Err(_) => location,
                        }
                    }
                    '1'..='8' => {
                        let x = i.to_digit(10).expect("This char should parse") as i8;
                        location = match move_board_location(&location, x, 0) {
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
                    'w' => PieceTeam::Light,
                    'b' => PieceTeam::Dark,
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

    /// Creates a new `GameState` representing the starting position of a chess game.
    ///
    /// # Returns
    /// * `Self` - Returns a `GameState` representing the starting position.
    pub fn new_game() -> Self {
        let new_game = String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        GameState::from_fen(&new_game).expect("New game string must have been corrupted")
    }

    /// Converts the current `GameState` to a FEN string.
    ///
    /// # Returns
    /// * `String` - Returns a FEN string representing the current `GameState`.
    pub fn get_fen(&self) -> String {
        let mut result = String::new();
        for i in (0..8).rev() {
            let mut space_count: u8 = 0;
            for j in 0..8 {
                if let Some(x) = self.piece_register.view(&(j, i)) {
                    if space_count > 0 {
                        result.push(space_count.to_string().chars().next().unwrap());
                    }
                    let c: char = match x.team {
                        PieceTeam::Light => match x.class {
                            PieceClass::Bishop => 'B',
                            PieceClass::King => 'K',
                            PieceClass::Knight => 'N',
                            PieceClass::Pawn => 'P',
                            PieceClass::Queen => 'Q',
                            PieceClass::Rook => 'R',
                        },
                        PieceTeam::Dark => match x.class {
                            PieceClass::Bishop => 'b',
                            PieceClass::King => 'k',
                            PieceClass::Knight => 'n',
                            PieceClass::Pawn => 'p',
                            PieceClass::Queen => 'q',
                            PieceClass::Rook => 'r',
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
            PieceTeam::Dark => result.push('b'),
            PieceTeam::Light => result.push('w'),
        };
        result.push(' ');

        if !self.can_castle_king_dark
            & !self.can_castle_queen_dark
            & !self.can_castle_king_light
            & !self.can_castle_queen_light
        {
            result.push('-');
        } else {
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

    /// Returns the material score for the current position.
    ///
    /// # Returns
    /// * `i8` - Positive favors light, negative favors dark.
    pub fn get_material_score(&self) -> i8 {
        let mut score = 0;
        for (_, piece_record) in self.piece_register.iter() {
            let piece_value = match piece_record.class {
                PieceClass::Pawn => 1,
                PieceClass::Knight => 3,
                PieceClass::Bishop => 3,
                PieceClass::Rook => 5,
                PieceClass::Queen => 9,
                PieceClass::King => 0, // Kings are ignored
            };
            score += match piece_record.team {
                PieceTeam::Light => piece_value,
                PieceTeam::Dark => -piece_value,
            };
        }
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Tests the creation of a new game and the conversion to and from FEN strings.
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
