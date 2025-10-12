use crate::board_location::{BoardLocation};
use crate::chess_errors::ChessErrors;
use crate::piece_register::PieceRegister;
use crate::piece_class::PieceClass;
use crate::piece_team::PieceTeam;
use crate::piece_record::PieceRecord;
use crate::scoring::conventional_score;
use crate::special_move_flags::SpecialMoveFlags;


/// The special stuff for castling rights and en passant
#[derive(Clone,Debug)]
pub struct MoveCounters{
    /// The half-move clock (for the 50-move rule).
    pub half_move_clock: u16,
    /// The full-move count (increments after black's move).
    pub full_move_count: u16,
}



/// Represents the complete state of a chess game at a given moment.
#[derive(Clone,Debug)]
pub struct GameState {
    /// The register containing all pieces and their locations.
    pub piece_register: PieceRegister,
    /// Game Status Flags
    pub special_flags : SpecialMoveFlags,
    /// Move couners
    pub move_counters : MoveCounters,
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
    pub fn from_fen(x: &str) -> Result<Self, ChessErrors> {
        let mut piece_register = PieceRegister::new();
        let mut can_castle_king_dark: bool = false;
        let mut can_castle_king_light: bool = false;
        let mut can_castle_queen_dark: bool = false;
        let mut can_castle_queen_light: bool = false;
        let mut en_passant_location = None;
        let half_move_clock: u16;
        let full_move_count: u16;
        let turn;

        let mut fields = x.split_ascii_whitespace();
        let mut file_index :u8 = 0;
        let mut rank_index :u8 = 7;
        if let Some(position_field) = fields.next() {

            for i in position_field.chars() {
                match i {
                    '/' => {
                            rank_index-=1;
                        },
                    '1'..='8' => {
                        let d = i.to_digit(10).expect("This char should parse") as u8;
                        file_index = (file_index + d)%8;
                    },
                    other =>{
                    let location = BoardLocation::from_file_rank(file_index, rank_index)?;
                    let new_piece = match other{
                    'p' => {
                        PieceRecord {
                            class: PieceClass::Pawn,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }                        
                    'r' => {
                        PieceRecord {
                            class: PieceClass::Rook,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }
                    'n' => {
                        PieceRecord {
                            class: PieceClass::Knight,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }
                    'b' => {
                          PieceRecord {
                            class: PieceClass::Bishop,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }
                    'q' => {
                        PieceRecord {
                            class: PieceClass::Queen,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }
                    'k' => {
                        PieceRecord {
                            class: PieceClass::King,
                            location,
                            team: PieceTeam::Dark,
                        }
                    }
                    'P' => {
                        PieceRecord {
                            class: PieceClass::Pawn,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    'R' => {
                        PieceRecord {
                            class: PieceClass::Rook,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    'N' => {
                        PieceRecord {
                            class: PieceClass::Knight,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    'B' => {
                        PieceRecord {
                            class: PieceClass::Bishop,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    'Q' => {
                        PieceRecord {
                            class: PieceClass::Queen,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    'K' => {
                        PieceRecord {
                            class: PieceClass::King,
                            location,
                            team: PieceTeam::Light,
                        }
                    }
                    token => {
                        return Err(ChessErrors::InvalidFENtoken(token));
                    }
                    };
                    piece_register.add_piece_record_no_rule_checking(new_piece);
                    file_index = (file_index + 1)%8;
                    }
                } 
            }    
        }  else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        if let Some(turn_field) = fields.next() {
            if let Some(c) = turn_field.chars().next() {
                turn = match c {
                    'w' => PieceTeam::Light,
                    'b' => PieceTeam::Dark,
                    x => return Err(ChessErrors::InvalidFENtoken(x)),
                }
            } else {
                return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
            }
        } else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        if let Some(castle_field) = fields.next() {
            for c in castle_field.chars() {
                match c {
                    'k' => can_castle_king_dark = true,
                    'q' => can_castle_queen_dark = true,
                    'K' => can_castle_king_light = true,
                    'Q' => can_castle_queen_light = true,
                    '-' => (),
                    x => return Err(ChessErrors::InvalidFENtoken(x)),
                }
            }
        } else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        if let Some(en_passant_field) = fields.next() {
            if en_passant_field != "-"{
            en_passant_location = Some(BoardLocation::from_long_algebraic(en_passant_field)?);
        }
        } else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        if let Some(half_move_clock_str) = fields.next() {
            if let Ok(c) = half_move_clock_str.parse::<u16>() {
                half_move_clock = c;
            } else {
                return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
            }
        } else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        if let Some(full_move_field_str) = fields.next() {
            if let Ok(c) = full_move_field_str.parse::<u16>() {
                full_move_count = c;
            } else {
                return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
            }
        } else {
            return Err(ChessErrors::InvalidFEDstringForm(x.to_string()));
        }

        Ok(GameState {
            piece_register,
            special_flags : SpecialMoveFlags { 
                can_castle_queen_light, 
                can_castle_king_light, 
                can_castle_queen_dark, 
                can_castle_king_dark, 
                en_passant_location},
            move_counters: MoveCounters { 
                half_move_clock, 
                full_move_count},
            turn
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
                if let Ok(x) = self.piece_register.view_piece_at_location(BoardLocation::from_file_rank(j, i).unwrap()) {
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

        if !self.special_flags.can_castle_king_dark
            & !self.special_flags.can_castle_queen_dark
            & !self.special_flags.can_castle_king_light
            & !self.special_flags.can_castle_queen_light
        {
            result.push('-');
        } else {
            if self.special_flags.can_castle_king_light {
                result.push('K');
            }
            if self.special_flags.can_castle_queen_light {
                result.push('Q');
            }
            if self.special_flags.can_castle_king_dark {
                result.push('k');
            }
            if self.special_flags.can_castle_queen_dark {
                result.push('q');
            }
        }
        result.push(' ');

        if let Some(loc) = self.special_flags.en_passant_location {
            let (rank,file) = loc.get_file_rank(); 
            match rank {
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
            match file {
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

        result.push_str(&self.move_counters.half_move_clock.to_string());
        result.push(' ');

        result.push_str(&self.move_counters.full_move_count.to_string());
        result
    }

    /// Returns the material score for the current position.
    ///
    /// # Returns
    /// * `i8` - Positive favors light, negative favors dark.
    pub fn get_material_score(&self) -> i8 {
        let mut score = 0;
        for piece_record in &self.piece_register.light_pieces {
            score += conventional_score(&piece_record.class) as i8;
        }
        for piece_record in &self.piece_register.dark_pieces {
            score -= conventional_score(&piece_record.class) as i8;       
        }     
        score
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Tests the creation of a new game and the conversion to and from FEN strings.
    fn test_fen_parsing() {
        let dut = GameState::new_game();
        let new_game_string =
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
        assert_eq!(dut.get_fen(), new_game_string);
        let score = dut.get_material_score();
        assert_eq!(score,0);

        let game_string =
            "1r4k1/7p/3p1bp1/p1pP4/P1P1prP1/1N2R2P/1P1N1PK1/8 b - - 3 31".to_string();
        let dut = GameState::from_fen(&game_string).expect("Should parse this string");
        assert_eq!(dut.get_fen(), game_string);
        let score = dut.get_material_score();
        assert_eq!(score,-1);

        let game_string =
            "r1bq1rk1/ppp2ppp/2n5/2bp4/4n3/1P2PNP1/PBP2PBP/RN1Q1RK1 b - - 2 9".to_string();
        let dut = GameState::from_fen(&game_string).expect("Should parse this string");
        assert_eq!(dut.get_fen(), game_string);
        let score = dut.get_material_score();
        assert_eq!(score,0);

        let game_string = "8/bpp1k2p/p2pP1p1/P5q1/1P5N/8/6PP/5Q1K b - - 0 35".to_string();
        let dut = GameState::from_fen(&game_string).expect("Should parse this string");
        assert_eq!(dut.get_fen(), game_string);
        let score = dut.get_material_score();
        assert_eq!(score,-1);
    }
}
