use crate::{board_location::BoardLocation, piece_types::PieceClass};

#[derive(Clone, Debug)]
pub enum MoveSpecialness {
    Regular,                  // Used for moving and capturing
    Promote(PieceClass),      // Class is type to promote
    EnPassant(BoardLocation), // BoardLocation is behind pawn,
    Castling(BoardLocation),  // Board location is for rook,
}

#[derive(Clone, Debug)]
pub struct ChessMove {
    pub start: BoardLocation,
    pub stop: BoardLocation,
    pub move_specialness: MoveSpecialness,
}

impl ChessMove {
    /// Converts this move description to long algebraic notation (e.g., "e2e4", "e7e8q").
    pub fn to_long_algebraic(&self) -> String {
        fn square_to_str(loc: &(i8, i8)) -> String {
            let file = (b'a' + loc.0 as u8) as char;
            let rank = (b'1' + loc.1 as u8) as char;
            format!("{}{}", file, rank)
        }
        let mut s = format!("{}{}", square_to_str(&self.start), square_to_str(&self.stop));
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
    pub fn from_long_algebraic(x: &str) -> Option<Self> {
        // Must be at least 4 chars (e.g., e2e4), up to 5 (e.g., e7e8q)
        let x = x.trim();
        if x.len() < 4 { return None; }
        let bytes = x.as_bytes();
        // Parse start square
        let file_from = bytes[0] as char;
        let rank_from = bytes[1] as char;
        let file_to = bytes[2] as char;
        let rank_to = bytes[3] as char;

        // Helper to convert file/rank to BoardLocation
        fn parse_square(file: char, rank: char) -> Option<BoardLocation> {
            let file_idx = match file {
                'a'..='h' => (file as u8 - b'a') as u8,
                _ => return None,
            };
            let rank_idx = match rank {
                '1'..='8' => (rank as u8 - b'1') as u8,
                _ => return None,
            };
            Some((file_idx as i8, rank_idx as i8))
        }

        let start = parse_square(file_from, rank_from)?;
        let stop = parse_square(file_to, rank_to)?;

        // Promotion? (e.g., e7e8q)
        let move_specialness = if x.len() == 5 {
            match bytes[4] as char {
                'q' | 'Q' => MoveSpecialness::Promote(PieceClass::Queen),
                'r' | 'R' => MoveSpecialness::Promote(PieceClass::Rook),
                'b' | 'B' => MoveSpecialness::Promote(PieceClass::Bishop),
                'n' | 'N' => MoveSpecialness::Promote(PieceClass::Knight),
                _ => return None,
            }
        } else {
            MoveSpecialness::Regular
        };

        // We can't know stop_occupancy from notation alone, so default to Empty
        Some(ChessMove {
            start,
            stop,
            move_specialness
        })
    }
}