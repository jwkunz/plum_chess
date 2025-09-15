use core::error;

use rand::Error;

use crate::{
    board_location::BoardLocation,
    errors::Errors,
    game_state::{self, GameState},
    piece_types::PieceClass,
};

#[derive(Clone, Debug)]
pub enum MoveSpecialness {
    Regular,                                  // Used for moving and capturing
    Promote(PieceClass),                      // Class is type to promote
    EnPassant(BoardLocation), // The current double step is vulnerable to en passant. BoardLocation is behind pawn to be captured,
    Castling((BoardLocation, BoardLocation)), // Board location is for rook (start,stop),
}

#[derive(Clone, Debug)]
pub struct ChessMove {
    pub start: BoardLocation,
    pub stop: BoardLocation,
    pub move_specialness: MoveSpecialness,
}

impl ChessMove {
    /// Converts this move description to long algebraic notation (e.g., "e2e4", "e7e8q").
    pub fn to_long_algebraic(&self, game: &GameState) -> String {
        fn square_to_str(loc: &(i8, i8)) -> String {
            let file = (b'a' + loc.0 as u8) as char;
            let rank = (b'1' + loc.1 as u8) as char;
            format!("{}{}", file, rank)
        }
        let mut s = format!(
            "{}{}",
            square_to_str(&self.start),
            square_to_str(&self.stop)
        );
        if let MoveSpecialness::Promote(pc) = &self.move_specialness {
            let promo = match pc {
                PieceClass::Queen => 'q',
                PieceClass::Rook => 'r',
                PieceClass::Bishop => 'b',
                PieceClass::Knight => 'n',
                // If other, default to 'q'
                _ => 'q',
            };
            s.push(promo);
        }
        s
    }
    /// Attempts to create a ChessMove from a long algebraic notation string (e.g., "e2e4", "e7e8q").
    /// Returns None if parsing fails.
    pub fn from_long_algebraic(game: &GameState, x: &str) -> Result<Self, Errors> {
        // Must be at least 4 chars (e.g., e2e4), up to 5 (e.g., e7e8q)
        let x = x.trim();
        if x.len() < 4 {
            return Err(Errors::InvalidAlgebraic);
        }
        let bytes = x.as_bytes();
        // Parse start square
        let file_from = bytes[0] as char;
        let rank_from = bytes[1] as char;
        let file_to = bytes[2] as char;
        let rank_to = bytes[3] as char;

        // Helper to convert file/rank to BoardLocation
        fn parse_square(file: char, rank: char) -> Result<BoardLocation, Errors> {
            let file_idx = match file {
                'a'..='h' => (file as u8 - b'a') as u8,
                _ => return Err(Errors::InvalidAlgebraic),
            };
            let rank_idx = match rank {
                '1'..='8' => (rank as u8 - b'1') as u8,
                _ => return Err(Errors::InvalidAlgebraic),
            };
            Ok((file_idx as i8, rank_idx as i8))
        }

        let start = parse_square(file_from, rank_from)?;
        let stop = parse_square(file_to, rank_to)?;

        // Figure out specialness
        let move_specialness = {
            if let Some(piece) = game.piece_register.view(&start) {
                // Is a promotion?
                if x.len() == 5 {
                    match bytes[4] as char {
                        'q' | 'Q' => MoveSpecialness::Promote(PieceClass::Queen),
                        'r' | 'R' => MoveSpecialness::Promote(PieceClass::Rook),
                        'b' | 'B' => MoveSpecialness::Promote(PieceClass::Bishop),
                        'n' | 'N' => MoveSpecialness::Promote(PieceClass::Knight),
                        _ => return Err(Errors::InvalidAlgebraic),
                    }
                } else if matches!(piece.class, PieceClass::King) {
                    // Castling detect
                    if x == "e1g1" && game.can_castle_king_light {
                        MoveSpecialness::Castling(((7, 0), (5, 0)))
                    } else if x == "e1c1" && game.can_castle_queen_light {
                        MoveSpecialness::Castling(((0, 0), (3, 0)))
                    } else if x == "e8g8" && game.can_castle_king_dark {
                        MoveSpecialness::Castling(((7, 7), (5, 7)))
                    } else if x == "e8c8" && game.can_castle_queen_dark {
                        MoveSpecialness::Castling(((0, 7), (3, 7)))
                    } else {
                        // Just a king move
                        MoveSpecialness::Regular
                    }
                } else {
                    // Not a king move

                    // Is a pawn en passant?
                    if matches!(piece.class, PieceClass::Pawn) && start.0 != stop.0 && game.piece_register.view(&stop).is_none(){
                        // The location next door for en passant
                        MoveSpecialness::EnPassant((stop.0, start.1))
                    } else {
                        // Just a regular move
                        MoveSpecialness::Regular
                    }
                }
            } else {
                // Move does not apply to a piece
                return Err(Errors::TryingToMoveNonExistantPiece);
            }
        };

        // We can't know stop_occupancy from notation alone, so default to Empty
        Ok(ChessMove {
            start,
            stop,
            move_specialness,
        })
    }
}
