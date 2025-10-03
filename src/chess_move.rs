use crate::{
    board_location::BoardLocation,
    errors::{ChessErrors},
    game_state::GameState,
    piece_types::{PieceClass, PieceRecord},
};

/// Represents special move types in chess, such as promotion, castling, en passant, and double pawn step.
/// Used to distinguish between regular moves and moves with special rules.
#[derive(Clone, Copy, Debug)]
pub enum MoveDescription {
    /// A regular move: (Piece start,stop)
    Regular(PieceRecord,PieceRecord),
    /// En passant capture (Piece start,stop,victim)
    EnPassant(PieceRecord,PieceRecord,PieceRecord),
    /// Castling move (King start,stop; Rook start,stop)
    Castling(PieceRecord, PieceRecord,PieceRecord, PieceRecord),
    /// Promotion to a specific piece (Piece start,stop)
    Promote(PieceRecord,PieceRecord),
    /// Double pawn step; (Pawn start,stop,vulnerable_square_behind)
    DoubleStep(PieceRecord,PieceRecord,BoardLocation),
    /// Capture move (capturing piece start, stop)
    Capture(PieceRecord,PieceRecord),
    /// Check (threatening piece start,stop,king piece)
    Check(PieceRecord,PieceRecord,PieceRecord),
    /// Check (threatening piece start,stop,king piece,other_threatening_piece)
    DoubleCheck(PieceRecord,PieceRecord,PieceRecord,PieceRecord),    
}

/// Converts this move description to long algebraic notation (e.g., "e2e4", "e7e8q").
///
/// # Arguments
/// * x - The current game state (not used in this function, but may be useful for context).
///
/// # Returns
/// * `String` - The move in long algebraic notation.
pub fn move_description_create_long_algebraic(move_description : MoveDescription) -> String{

} 

pub fn move_description_from_long_algebraic(long_algebraic_string : &str, game : &GameState) -> Result<MoveDescription,ChessErrors>{

} 

impl ChessMove {
    /// Converts this move description to long algebraic notation (e.g., "e2e4", "e7e8q").
    ///
    /// # Arguments
    /// * `_game` - The current game state (not used in this function, but may be useful for context).
    ///
    /// # Returns
    /// * `String` - The move in long algebraic notation.
    pub fn to_long_algebraic(&self, _game: &GameState) -> String {
        /// Helper function to convert a board location to algebraic notation (e.g., (4,1) -> "e2").
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
        // Add promotion piece if this is a promotion move.
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
    ///
    /// # Arguments
    /// * `game` - The current game state (used to determine move specialness).
    /// * `x` - The move in long algebraic notation.
    ///
    /// # Returns
    /// * `Ok(ChessMove)` if parsing is successful.
    /// * `Err(Errors)` if parsing fails or the move is invalid.
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

        /// Helper to convert file/rank chars to BoardLocation.
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

        // Determine the specialness of the move based on the piece and notation.
        let move_specialness = {
            if let Some(piece) = game.piece_register.view(&start) {
                // Is this a promotion?
                if x.len() == 5 {
                    match bytes[4] as char {
                        'q' | 'Q' => MoveSpecialness::Promote(PieceClass::Queen),
                        'r' | 'R' => MoveSpecialness::Promote(PieceClass::Rook),
                        'b' | 'B' => MoveSpecialness::Promote(PieceClass::Bishop),
                        'n' | 'N' => MoveSpecialness::Promote(PieceClass::Knight),
                        _ => return Err(Errors::InvalidAlgebraic),
                    }
                } else if matches!(piece.class, PieceClass::King) {
                    // Detect castling based on notation and castling rights.
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

                    // Is this a pawn en passant capture?
                    if matches!(piece.class, PieceClass::Pawn) && start.0 != stop.0 && game.piece_register.view(&stop).is_none(){
                        // The location next door for en passant
                        MoveSpecialness::EnPassant((stop.0, start.1))
                    } else {
                        // Just a regular move
                        MoveSpecialness::Regular
                    }
                }
            } else {
                // No piece at the start location.
                return Err(Errors::TryingToMoveNonExistantPiece((start,game.get_fen())));
            }
        };

        // We can't know stop_occupancy from notation alone, so default to Empty.
        Ok(ChessMove {
            start,
            stop,
            move_specialness,
        })
    }
}
