use std::{collections::LinkedList, io::ErrorKind};

use crate::{
    errors::Errors,
    game_state::{self, GameState},
    types::{move_board_location, BoardLocation, PieceClass, PieceTeam},
};

#[derive(Clone, Debug)]
pub enum MoveSpecialness {
    Regular,                  // Used for moving and capturing
    Promote(PieceClass),      // Class is type to promote
    EnPassant(BoardLocation), // BoardLocation is behind pawn,
    Castling(BoardLocation),  // Board location is for rook,
}

#[derive(Clone, Debug)]
pub enum Occupancy {
    Empty,        // Empty square
    EnemyRegular, // Non-king occupied square of other side
    EnemyKing,    // King occupied square of other side
}

#[derive(Clone, Debug)]
pub struct ChessMoveDescription {
    pub start: BoardLocation,
    pub stop: BoardLocation,
    pub move_specialness: MoveSpecialness,
    pub stop_occupancy: Occupancy,
}

type ListOfMoves = LinkedList<ChessMoveDescription>;

/// This function will apply the chess_move to a game to create a new game state
pub fn apply_move_to_game(game: &GameState, chess_move: &ChessMoveDescription) -> GameState {
    game.clone()
}

// This function checks if a given game state allows capturing an enemy king in the given turn
// Depending on whose turn is active this can be used to inspect for "check"
fn can_capture_enemy_king(game: &GameState) -> bool {
    true
}

// Returns the vector for forward depending on whose turn it is
fn get_forward_direction_for_turn(turn: &PieceTeam) -> i8 {
    match turn {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    }
}

// Checks if a location is given piece type
fn verify_is_piece(
    game: &GameState,
    location: &BoardLocation,
    class: PieceClass,
) -> Result<(), Errors> {
    if let Some(x) = game.piece_register.view(location) {
        if std::mem::discriminant(&class) == std::mem::discriminant(&x.class) {
            Ok(())
        } else {
            Err(Errors::MoveStartLocationIsNotValidPieceType)
        }
    } else {
        Err(Errors::MoveStartLocationIsNotValidPieceType)
    }
}

// Considers start and stop as a potential move
// and if feasible will add it to result with appropriate context
// Returns true if something was added, false if not
// Does not inspect rules for check
fn try_add_move_pawn(
    game: &GameState,
    start: &BoardLocation,
    stop: &BoardLocation,
    result: &mut ListOfMoves,
) -> bool {
    // Assume no collision occurs
    let mut occupancy_type = Occupancy::Empty;
    // Unless
    if let Some(target) = game.piece_register.view(stop) {
        // Something was at the stop location

        // Look if the move was forward
        if start.0 == stop.0 {
            // Cannot capture moving forward
            return false;
        }

        // What kind of piece collision was it?
        occupancy_type = if game.turn == target.team {
            return false; // Collide with teammate, not a move
        } else {
            match target.class {
                PieceClass::King => Occupancy::EnemyKing,
                _ => Occupancy::EnemyRegular,
            }
        };
    } else {
        // Look if the move was diagonal
        if start.0 != stop.0 {
            // Cannot move diagonal without capture
            return false;
        }
    }
    // Look for promotion on back rank
    if (game.turn == PieceTeam::Light && stop.1 == 7)
        || (game.turn == PieceTeam::Dark && stop.1 == 0)
    {
        // All the kinds of promotions
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_specialness: MoveSpecialness::Promote(PieceClass::Queen),
            stop_occupancy: occupancy_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_specialness: MoveSpecialness::Promote(PieceClass::Rook),
            stop_occupancy: occupancy_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_specialness: MoveSpecialness::Promote(PieceClass::Bishop),
            stop_occupancy: occupancy_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_specialness: MoveSpecialness::Promote(PieceClass::Knight),
            stop_occupancy: occupancy_type.clone(),
        });
        true
    } else {
        // A regular move (capture or movement)

        // Check for en passant move
        let move_specialness = if (stop.1 - start.1).abs() == 2 {
            let en_passant_square = (start.0, (start.1 + stop.1) / 2);
            if game.piece_register.view(&en_passant_square).is_some() {
                // Can't jump over a piece in the en passant spot
                return false;
            }
            MoveSpecialness::EnPassant(en_passant_square)
        } else {
            MoveSpecialness::Regular
        };

        // Add move
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_specialness,
            stop_occupancy: occupancy_type.clone(),
        });
        true
    }
}

// Generates all possible move before evaluating for check
pub fn generate_potential_moves_pawn(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();

    // Check if start location piece is actually a pawn
    verify_is_piece(game, start, PieceClass::Pawn)?;

    // Mark the forward direction
    let forward_direction = get_forward_direction_for_turn(&game.turn);

    // Try diagonal captures
    if let Ok(stop) = move_board_location(start, 1, forward_direction) {
        try_add_move_pawn(game, start, &stop, &mut result);
    }
    if let Ok(stop) = move_board_location(start, -1, forward_direction) {
        try_add_move_pawn(game, start, &stop, &mut result);
    }

    // Try forward march
    if let Ok(stop) = move_board_location(start, 0, forward_direction) {
        try_add_move_pawn(game, start, &stop, &mut result);
    }

    // Try en passant first move
    let start_square = match game.turn {
        PieceTeam::Dark => 6,
        PieceTeam::Light => 1,
    };
    if start_square == start.1 {
        if let Ok(stop) = move_board_location(start, 0, 2 * forward_direction) {
            try_add_move_pawn(game, start, &stop, &mut result);
        }
    }
    // Return whatever was available
    Ok(result)
}

// Considers start and stop as a potential move
// and if feasible will add it to result with appropriate context
// Returns true if something was added, false if not
// Does not inspect rules for check
fn try_add_move_generic(
    game: &GameState,
    start: &BoardLocation,
    stop: &BoardLocation,
    result: &mut ListOfMoves,
) -> bool {
    // Assume no collision occurs
    let mut occupancy_type = Occupancy::Empty;
    // Unless
    if let Some(target) = game.piece_register.view(stop) {
        // Something was at the stop location

        // What kind of piece collision was it?
        occupancy_type = if game.turn == target.team {
            return false; // Collide with teammate, not a move
        } else {
            match target.class {
                PieceClass::King => Occupancy::EnemyKing,
                _ => Occupancy::EnemyRegular,
            }
        };
    }

    // A regular move (capture or movement)
    // Add move
    result.push_back(ChessMoveDescription {
        start: *start,
        stop: *stop,
        move_specialness: MoveSpecialness::Regular,
        stop_occupancy: occupancy_type.clone(),
    });
    true
}

pub fn generate_potential_moves_knight(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a knight
    verify_is_piece(game, start, PieceClass::Knight)?;
    // Try all 8 knight moves
    if let Ok(stop) = move_board_location(start, 2, 1) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, 2, -1) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, -2, 1) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, -2, -1) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, 1, 2) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, -1, 2) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, 1, -2) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    if let Ok(stop) = move_board_location(start, -1, -2) {
        try_add_move_generic(game, start, &stop, &mut result);
    };
    Ok(result)
}
pub fn generate_potential_moves_bishop(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_potential_moves_rook(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_potential_moves_queen(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_potential_moves_king(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}

/// This function get's all possible moves for a given turn
pub fn generate_all_moves(game: &GameState) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Go through all squares
    for (location, piece_record) in game.piece_register.iter() {
        // If the piece belongs to the current turn
        if piece_record.team == game.turn {
            // Generate all the potential moves
            let potential_moves = match piece_record.class {
                crate::types::PieceClass::Pawn => generate_potential_moves_pawn(game, &location)?,
                crate::types::PieceClass::Knight => {
                    generate_potential_moves_knight(game, &location)?
                }
                crate::types::PieceClass::Bishop => {
                    generate_potential_moves_bishop(game, &location)?
                }
                crate::types::PieceClass::Rook => generate_potential_moves_rook(game, &location)?,
                crate::types::PieceClass::Queen => generate_potential_moves_queen(game, &location)?,
                crate::types::PieceClass::King => generate_potential_moves_king(game, &location)?,
            };
            // Make sure potential moves don't violate rules
            for k in potential_moves {
                let trial_game = apply_move_to_game(game, &k);
                // Verify that move did not allow a capture of king
                if can_capture_enemy_king(&trial_game) {
                    continue;
                }
                // All rules validated
                result.push_back(k);
            }
        }
    }
    // Return result
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pawn_moves() -> Result<(), Errors> {
        let test_game = GameState::from_fen("3k4/8/8/8/8/8/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 2);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3p4/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 3);

        let test_game = GameState::from_fen("3k4/4P3/8/8/8/8/8/3K4 w - - 0 1").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(4, 6))?;
        assert_eq!(moves.len(), 8);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3pP3/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 1);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3pP3/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(4, 2))?;
        assert_eq!(moves.len(), 1);

        let test_game =
            GameState::from_fen("6k1/7p/3p2p1/p2P4/P1PpprP1/1r5P/1P1N1PK1/8 w - - 0 34").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(1, 1))?;
        assert_eq!(moves.len(), 0);

        // TODO test this case once check logic is implemented "3k4/8/8/8/6b1/3pP3/4P3/3K4 w - - 0 1"

        Ok(())
    }

    #[test]
    fn test_knight_moves() -> Result<(), Errors> {
        let test_game =
            GameState::from_fen("6k1/7p/3p2p1/p2P4/P1PpprP1/1r5P/1P1N1PK1/8 w - - 0 34").unwrap();
        let moves = generate_potential_moves_knight(&test_game, &(3, 1))?;
        assert_eq!(moves.len(), 5);

        let test_game =
            GameState::from_fen("1r1b2k1/7p/3p2p1/p1pP4/P1P1prP1/1NR2N1P/1P3PK1/8 w - - 0 30")
                .unwrap();
        let moves = generate_potential_moves_knight(&test_game, &(5, 2))?;
        assert_eq!(moves.len(), 8);
        Ok(())
    }
}
