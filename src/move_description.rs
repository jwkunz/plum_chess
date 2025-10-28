use crate::{
    board_location::{BoardLocation},
    chess_errors::ChessErrors,
    game_state::GameState,
    piece_class::PieceClass,
    piece_record::PieceRecord,
};

/// Used for describing a change
#[derive(Clone, Copy, Debug)]
pub struct MoveVector {
    pub piece_at_start: PieceRecord,
    pub destination: BoardLocation,
}

/// Represents the move types in chess, such as promotion, castling, en passant, and double pawn step.
/// Used to distinguish between regular moves and moves with special rules and information
#[derive(Clone, Copy, Debug)]
pub enum MoveTypes {
    /// A regular move or regular capture
    Regular,
    /// En passant capture.  The capture_status contains the victim piece.
    EnPassant,
    /// Castling move (Rook vector)
    Castling(MoveVector),
    /// Promotion to a specific piece (Piece after promotion)
    Promote(PieceRecord),
    /// Double pawn step; (Vulnerable square left behind)
    DoubleStep(BoardLocation),
}

/// Represents the move types in chess, such as promotion, castling, en passant, and double pawn step.
/// Used to distinguish between regular moves and moves with special rules.
#[derive(Clone, Debug)]
pub struct MoveDescription {
    pub vector: MoveVector,
    pub move_type: MoveTypes,
    pub capture_status: Option<PieceRecord>,
}



impl MoveDescription {
    /// Converts this move description to long algebraic notation (e.g., "e2e4", "e7e8q").
    ///
    /// # Arguments
    /// * m - MoveDescription
    ///
    /// # Returns
    /// * `String` - The move in long algebraic notation.
    pub fn get_long_algebraic(&self) -> String {
        let base = format!(
            "{:}{:}",
            self.vector.piece_at_start.location.to_long_algebraic(),
            self.vector.destination.to_long_algebraic()
        );
        if let MoveTypes::Promote(p) = self.move_type {
            let promotion = match p.class {
                PieceClass::Queen => 'q',
                PieceClass::Rook => 'r',
                PieceClass::Bishop => 'b',
                _ => 'n',
            };
            format!("{:}{:}", base, promotion)
        } else {
            base
        }
    }

    /// Attempts to create a ChessMove from a long algebraic notation string (e.g., "e2e4", "e7e8q").
    ///
    /// # Arguments
    /// * `long_algebraic_str` - The move in long algebraic notation.
    /// * `game` - The current game state (used to determine move specialness).
    ///
    /// # Returns
    /// * `Ok(ChessMove)` if parsing is successful.
    /// * `Err(Errors)` if parsing fails or the move is invalid.
    pub fn from_long_algebraic(long_algebraic_str: &str, game: &GameState) -> Result<Self, ChessErrors> {
        // Must be at least 4 chars (e.g., e2e4), up to 5 (e.g., e7e8q)
        let x = long_algebraic_str.trim();
        if x.len() < 4 {
            return Err(ChessErrors::InvalidAlgebraicString(long_algebraic_str.to_string()));
        }
        let bytes = x.as_bytes();
        for i in [0,2]{
            let b = bytes[i];
            if b < 97 || b > 104{
                return Err(ChessErrors::InvalidAlgebraicChar(b as char))
            }
        }
        for i in [1,3]{
            let b = bytes[i];
            if b < 49 || b > 56{
                return Err(ChessErrors::InvalidAlgebraicChar(b as char))
            }
        }        
        // Parse start square
        let file_from = bytes[0]-'a' as u8;
        let rank_from = bytes[1]-1-'0' as u8;
        let file_to = bytes[2]-'a' as u8;
        let rank_to = bytes[3]-1-'0' as u8;

        let start = BoardLocation::from_file_rank(file_from, rank_from)?;
        let destination = BoardLocation::from_file_rank(file_to, rank_to)?;
        let piece_at_start = *game.piece_register.view_piece_at_location(start)?;
        let vector = MoveVector {piece_at_start, destination};
        let mut capture_status = if let Ok(x) = game.piece_register.view_piece_at_location(destination){
            Some(*x)
        }else{
            None
        };

        // Determine the type of the move based on the piece and notation.
        let move_type = {
            
            // Is this a promotion?
            if x.len() == 5 {
                match bytes[4] as char {
                    'q' | 'Q' => MoveTypes::Promote(PieceRecord {
                        class: PieceClass::Queen,
                        location: destination,
                        team: piece_at_start.team,
                    }),
                    'r' | 'R' => MoveTypes::Promote(PieceRecord {
                        class: PieceClass::Rook,
                        location: destination,
                        team: piece_at_start.team,
                    }),
                    'b' | 'B' => MoveTypes::Promote(PieceRecord {
                        class: PieceClass::Bishop,
                        location: destination,
                        team: piece_at_start.team,
                    }),
                    'n' | 'N' => MoveTypes::Promote(PieceRecord {
                        class: PieceClass::Knight,
                        location: destination,
                        team: piece_at_start.team,
                    }),
                    _ => return Err(ChessErrors::InvalidAlgebraicString(long_algebraic_str.to_string())),
                }
            } else if matches!(piece_at_start.class, PieceClass::King) {
                // Detect castling based on notation and castling rights.
                if x == "e1g1" && game.special_flags.can_castle_king_light {
                    MoveTypes::Castling(MoveVector {
                        // Get the rook
                        piece_at_start: *game
                            .piece_register
                            .view_piece_at_location(BoardLocation::from_file_rank(7, 0)?)?,
                        // Rook's destination
                        destination: BoardLocation::from_file_rank(5, 0)?,
                    })
                } else if x == "e1c1" && game.special_flags.can_castle_queen_light {
                    MoveTypes::Castling(MoveVector {
                        // Get the rook
                        piece_at_start: *game
                            .piece_register
                            .view_piece_at_location(BoardLocation::from_file_rank(0, 0)?)?,
                        // Rook's destination
                        destination: BoardLocation::from_file_rank(3, 0)?,
                    })
                } else if x == "e8g8" && game.special_flags.can_castle_king_dark {
                    MoveTypes::Castling(MoveVector {
                        // Get the rook
                        piece_at_start: *game
                            .piece_register
                            .view_piece_at_location(BoardLocation::from_file_rank(7, 7)?)?,
                        // Rook's destination
                        destination: BoardLocation::from_file_rank(5, 7)?,
                    })
                } else if x == "e8c8" && game.special_flags.can_castle_queen_dark {
                    MoveTypes::Castling(MoveVector {
                        // Get the rook
                        piece_at_start: *game
                            .piece_register
                            .view_piece_at_location(BoardLocation::from_file_rank(0, 7)?)?,
                        // Rook's destination
                        destination: BoardLocation::from_file_rank(3, 7)?,
                    })
                } else {
                    // Just a king move
                    MoveTypes::Regular
                }
            } else {
                // Not a king move

                // Is this a pawn en passant capture or double-step?
                if matches!(piece_at_start.class, PieceClass::Pawn) {
                    // Pawn moved diagonally into an empty square -> en passant capture
                    if file_from != file_to && capture_status.is_none() {
                        capture_status = Some(*game
                            .piece_register
                            .view_piece_at_location(BoardLocation::from_file_rank(file_to, rank_from)?)?);
                        MoveTypes::EnPassant
                    } else {
                        // Pawn moved two ranks -> double step (vulnerable square is midway)
                        let rank_diff = (rank_from as i8 - rank_to as i8).abs();
                        if rank_diff >= 2 {
                            MoveTypes::DoubleStep(BoardLocation::from_file_rank(
                                file_to,
                                (rank_from + rank_to) / 2,
                            )?)
                        } else {
                            // Regular pawn move or capture
                            MoveTypes::Regular
                        }
                    }
                } else {
                    // Non-pawn moves are regular unless handled elsewhere
                    MoveTypes::Regular
                }
            }
        };

        // We can't know stop_occupancy from notation alone, so default to Empty.
        Ok(MoveDescription { vector, move_type, capture_status})
    }
}


#[cfg(test)]
mod test{
    use crate::piece_team::PieceTeam;

    use super::*;
    #[test]
    fn test_moves_1(){
        let new_game = GameState::new_game();
        let move_text = "e2e4";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        assert!(matches!(move_description.vector.piece_at_start.class,PieceClass::Pawn));
        assert!(matches!(move_description.vector.piece_at_start.team,PieceTeam::Light));
        assert!(matches!(move_description.move_type,MoveTypes::DoubleStep(_)));
        assert!(matches!(move_description.capture_status,None));
    }
}
