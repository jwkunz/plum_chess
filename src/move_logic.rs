use std::collections::LinkedList;

use crate::{
    board_location::{BoardLocation},
    move_description::{MoveDescription},
    chess_errors::ChessErrors,
    game_state::{GameState},
    piece_class::PieceClass,
    piece_team::PieceTeam,
};


/// Type alias for a linked list of move descriptions with collision information.
type ListOfUncheckedMoves = LinkedList<MoveDescription>;

/// Checks if the current player can capture the enemy king in the given game state.
/// Used to determine if the current player is giving check.
///
/// # Arguments
/// * `game` - The game state to check.
///
/// # Returns
/// * `Ok(true)` if the enemy king can be captured.
/// * `Ok(false)` otherwise.
/// * `Err(Errors)` if move generation fails.
fn can_capture_enemy_king(game: &GameState) -> Result<bool, ChessErrors> {
    // Iterate over all pieces of the current player.
    for (location, piece_record) in game.piece_register.iter() {
        if piece_record.team == game.turn {
            // Generate all potential moves for this piece.
            let potential_moves = match piece_record.class {
                crate::piece_types::PieceClass::Pawn => {
                    generate_potential_moves_pawn(game, &location)?
                }
                crate::piece_types::PieceClass::Knight => {
                    generate_potential_moves_knight(game, &location)?
                }
                crate::piece_types::PieceClass::Bishop => {
                    generate_potential_moves_bishop(game, &location)?
                }
                crate::piece_types::PieceClass::Rook => {
                    generate_potential_moves_rook(game, &location)?
                }
                crate::piece_types::PieceClass::Queen => {
                    generate_potential_moves_queen(game, &location)?
                }
                crate::piece_types::PieceClass::King => {
                    generate_potential_moves_king(game, &location)?
                }
            };
            // Check if any move would capture the enemy king.
            for k in potential_moves {
                if matches!(k.stop_occupancy, Occupancy::EnemyKing) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Returns the forward direction for the given team.
/// Light moves up (+1), Dark moves down (-1).
///
/// # Arguments
/// * `turn` - The team whose direction is needed.
///
/// # Returns
/// * `1` for Light, `-1` for Dark.
fn get_forward_direction_for_turn(turn: &PieceTeam) -> i8 {
    match turn {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    }
}

/// Verifies that the piece at the given location is of the specified class and belongs to the current turn.
///
/// # Arguments
/// * `game` - The game state.
/// * `location` - The board location to check.
/// * `class` - The expected piece class.
///
/// # Returns
/// * `Ok(())` if the piece matches.
/// * `Err(Errors::InvalidMoveStartCondition)` otherwise.
fn verify_is_piece_class_and_turn(
    game: &GameState,
    location: &BoardLocation,
    class: PieceClass,
) -> Result<(), Errors> {
    if let Some(x) = game.piece_register.view(location) {
        if std::mem::discriminant(&class) == std::mem::discriminant(&x.class) {
            if std::mem::discriminant(&game.turn) == std::mem::discriminant(&x.team) {
                Ok(())
            } else {
                Err(Errors::InvalidMoveStartCondition)
            }
        } else {
            Err(Errors::InvalidMoveStartCondition)
        }
    } else {
        Err(Errors::InvalidMoveStartCondition)
    }
}

/// Attempts to add a pawn move to the result list, considering captures, promotions, and double steps.
/// Does not check for check or en passant (handled elsewhere).
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the pawn.
/// * `stop` - The destination location.
/// * `result` - The list to which the move will be added.
///
/// # Returns
/// * `true` if a move was added, `false` otherwise
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
        result.push_back(ChessMoveDescriptionWithCollision {
            description: ChessMove {
                start: *start,
                stop: *stop,
                move_specialness: MoveSpecialness::Promote(PieceClass::Queen),
            },
            stop_occupancy: occupancy_type.clone(),
        });
        // All the kinds of promotions
        result.push_back(ChessMoveDescriptionWithCollision {
            description: ChessMove {
                start: *start,
                stop: *stop,
                move_specialness: MoveSpecialness::Promote(PieceClass::Rook),
            },
            stop_occupancy: occupancy_type.clone(),
        });
        // All the kinds of promotions
        result.push_back(ChessMoveDescriptionWithCollision {
            description: ChessMove {
                start: *start,
                stop: *stop,
                move_specialness: MoveSpecialness::Promote(PieceClass::Bishop),
            },
            stop_occupancy: occupancy_type.clone(),
        });
        // All the kinds of promotions
        result.push_back(ChessMoveDescriptionWithCollision {
            description: ChessMove {
                start: *start,
                stop: *stop,
                move_specialness: MoveSpecialness::Promote(PieceClass::Knight),
            },
            stop_occupancy: occupancy_type.clone(),
        });
        true
    } else {
        // A regular move (capture or movement)

        
        let move_specialness = 
        // Check for double step move
        if (stop.1 - start.1).abs() == 2 {
            // Is a double step
            let en_passant_square = (start.0, (start.1 + stop.1) / 2); // Right behind
            if game.piece_register.view(&en_passant_square).is_some() {
                // Can't jump over a piece in the en passant spot
                return false;
            }
            MoveSpecialness::DoubleStep(en_passant_square)
        }
        else {
            MoveSpecialness::Regular
        };

        // Add move
        result.push_back(ChessMoveDescriptionWithCollision {
            description: ChessMove {
                start: *start,
                stop: *stop,
                move_specialness,
            },
            stop_occupancy: occupancy_type.clone(),
        });
        true
    }
}

/// Generates all possible moves for a pawn, including captures, promotions, double steps, and en passant.
/// Does not check for check.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the pawn.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible pawn moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_pawn(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();

    // Check if start location piece is actually a pawn
    verify_is_piece_class_and_turn(game, start, PieceClass::Pawn)?;

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

    // Try double step first move
    let start_square = match game.turn {
        PieceTeam::Dark => 6,
        PieceTeam::Light => 1,
    };
    if start_square == start.1 {
        if let Ok(stop) = move_board_location(start, 0, 2 * forward_direction) {
            try_add_move_pawn(game, start, &stop, &mut result);
        }
    }

    // Manually try to add en passant
    if let Some(behind_pawn) = game.en_passant_location{
        // Behind pawn is diagonal to this pawn and no piece is there
        if (start.1 + forward_direction == behind_pawn.1) && ((behind_pawn.0-start.0).abs() == 1) && game.piece_register.view(&behind_pawn).is_none(){
            // Add en passant
            result.push_back(
                ChessMoveDescriptionWithCollision{
                    description:ChessMove{
                        start:*start,
                        stop:behind_pawn,
                        move_specialness:MoveSpecialness::EnPassant(behind_pawn)},
                    stop_occupancy:Occupancy::Empty});
        }
    }

    // Return whatever was available
    Ok(result)
}

/// Generates all possible moves for a knight from the given location.
/// Does not check for check.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the knight.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible knight moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_knight(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a knight
    verify_is_piece_class_and_turn(game, start, PieceClass::Knight)?;
    // Try all 8 knight moves
    if let Ok(stop) = move_board_location(start, 2, 1) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, 2, -1) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, -2, 1) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, -2, -1) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, 1, 2) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, -1, 2) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, 1, -2) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    if let Ok(stop) = move_board_location(start, -1, -2) {
        if let Some(x) = check_move_collision(game, start, &stop) {
            result.push_back(x);
        }
    };
    Ok(result)
}

/// Generates all possible moves for a bishop from the given location.
/// Does not check for check.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the bishop.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible bishop moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_bishop(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a bishop
    verify_is_piece_class_and_turn(game, start, PieceClass::Bishop)?;
    // Try all 4 bishop directions until collision
    // Up right
    follow_move_vector(game, start, 1, 1, &mut result);
    // Down right
    follow_move_vector(game, start, -1, 1, &mut result);
    // Up left
    follow_move_vector(game, start, 1, -1, &mut result);
    // Down left
    follow_move_vector(game, start, -1, -1, &mut result);
    // Return
    Ok(result)
}

/// Generates all possible moves for a rook from the given location.
/// Does not check for check.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the rook.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible rook moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_rook(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a bishop
    verify_is_piece_class_and_turn(game, start, PieceClass::Rook)?;
    // Try all 4 rook directions until collision
    // Up
    follow_move_vector(game, start, 1, 0, &mut result);
    // Down
    follow_move_vector(game, start, -1, 0, &mut result);
    // Left
    follow_move_vector(game, start, 0, -1, &mut result);
    // Right
    follow_move_vector(game, start, 0, 1, &mut result);
    // Return
    Ok(result)
}

/// Generates all possible moves for a queen from the given location.
/// Does not check for check.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the queen.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible queen moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_queen(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a queen
    verify_is_piece_class_and_turn(game, start, PieceClass::Queen)?;
    // Try all 4 rook directions until collision
    // Up
    follow_move_vector(game, start, 1, 0, &mut result);
    // Down
    follow_move_vector(game, start, -1, 0, &mut result);
    // Left
    follow_move_vector(game, start, 0, -1, &mut result);
    // Right
    follow_move_vector(game, start, 0, 1, &mut result);
    // Try all 4 bishop directions until collision
    // Up right
    follow_move_vector(game, start, 1, 1, &mut result);
    // Down right
    follow_move_vector(game, start, -1, 1, &mut result);
    // Up left
    follow_move_vector(game, start, 1, -1, &mut result);
    // Down left
    follow_move_vector(game, start, -1, -1, &mut result);
    // Return
    Ok(result)
}

/// Generates all possible moves for a king from the given location, including castling if legal.
/// Checks for check when considering castling.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location of the king.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All possible king moves.
/// * `Err(Errors)` - If the move is invalid.
pub fn generate_potential_moves_king(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Check if start location piece is actually a king
    verify_is_piece_class_and_turn(game, start, PieceClass::King)?;
    // Try all 8 king moves
    for i in -1..2 {
        for j in -1..2 {
            if (i == 0) && (j == 0) {
                continue;
            }
            if let Ok(stop) = move_board_location(start, i, j) {
                if let Some(x) = check_move_collision(game, start, &stop) {
                    result.push_back(x);
                }
            };
        }
    }
    // Try to add castling
  
    if (game.can_castle_king_dark && matches!(game.turn,PieceTeam::Dark)) || (game.can_castle_king_light && matches!(game.turn,PieceTeam::Light)){
        // Make sure spaces are empty
        if game.piece_register.view(&(start.0+1,start.1)).is_none()
        && game.piece_register.view(&(start.0+2,start.1)).is_none(){
            // Try to move king across and see if there is check
            let mut in_check = false;
            for i in 0..=1{
                let temp_game = apply_move_to_game(&game, &ChessMove{start: *start, stop: (start.0+i,start.1), move_specialness:MoveSpecialness::Regular})?;
                in_check |= can_capture_enemy_king(&temp_game)?
            }
            if !in_check{
                result.push_back(ChessMoveDescriptionWithCollision { description: ChessMove { start: *start, stop: (start.0+2,start.1), move_specialness: MoveSpecialness::Castling(((start.0+3,start.1),(start.0+1,start.1))) }, stop_occupancy: Occupancy::Empty });
            }
        }
    }
    if (game.can_castle_queen_dark && matches!(game.turn,PieceTeam::Dark)) || (game.can_castle_queen_light && matches!(game.turn,PieceTeam::Light)){
        // Make sure spaces are empty
        if game.piece_register.view(&(start.0-1,start.1)).is_none()
        && game.piece_register.view(&(start.0-2,start.1)).is_none()
        && game.piece_register.view(&(start.0-3,start.1)).is_none(){
            // Try to move king across and see if there is check
            let mut in_check = false;
            for i in 0..=2{
                let temp_game = apply_move_to_game(&game, &ChessMove{start: *start, stop: (start.0-i,start.1), move_specialness:MoveSpecialness::Regular})?;
                in_check |= can_capture_enemy_king(&temp_game)?
            }
            if !in_check{
                result.push_back(ChessMoveDescriptionWithCollision { description: ChessMove { start: *start, stop: (start.0-2,start.1), move_specialness: MoveSpecialness::Castling(((start.0-4,start.1),(start.0-1,start.1))) }, stop_occupancy: Occupancy::Empty });
            }
        }
    }
    

    Ok(result)
}

/// Checks if a move from `start` to `stop` is legal based on collision rules.
/// Returns a move description with collision info if legal, or `None` if not.
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location.
/// * `stop` - The destination location.
///
/// # Returns
/// * `Some(ChessMoveDescriptionWithCollision)` if the move is legal.
/// * `None` if the move is not legal (e.g., blocked by teammate).
fn check_move_collision(
    game: &GameState,
    start: &BoardLocation,
    stop: &BoardLocation,
) -> Option<ChessMoveDescriptionWithCollision> {
    // Assume no collision occurs
    let mut occupancy_type = Occupancy::Empty;
    // Unless
    if let Some(target) = game.piece_register.view(stop) {
        // Something was at the stop location

        // What kind of piece collision was it?
        occupancy_type = if game.turn == target.team {
            return None; // Collide with teammate, not a move
        } else {
            match target.class {
                PieceClass::King => Occupancy::EnemyKing,
                _ => Occupancy::EnemyRegular,
            }
        };
    }
    Some(ChessMoveDescriptionWithCollision {
        description: ChessMove {
            start: *start,
            stop: *stop,
            move_specialness: MoveSpecialness::Regular,
        },
        stop_occupancy: occupancy_type.clone(),
    })
}

/// Follows a move vector (dx, dy) from a starting location, adding all legal moves along the vector until blocked.
/// Used for sliding pieces (bishop, rook, queen).
///
/// # Arguments
/// * `game` - The game state.
/// * `start` - The starting location.
/// * `dx` - The x-direction increment.
/// * `dy` - The y-direction increment.
/// * `result` - The list to which moves will be added.
fn follow_move_vector(
    game: &GameState,
    start: &BoardLocation,
    dx: i8,
    dy: i8,
    result: &mut LinkedList<ChessMoveDescriptionWithCollision>,
) {
    for distance in 1..8 {
        if let Ok(stop) = move_board_location(start, dx * distance, dy * distance) {
            if let Some(x) = check_move_collision(game, start, &stop) {
                match x.stop_occupancy {
                    Occupancy::Empty => result.push_back(x),
                    _ => {
                        result.push_back(x);
                        break;
                    }
                }
            } else {
                break;
            }
        } else {
            break;
        };
    }
}

/// Generates all legal moves for the current player in the given game state.
/// Filters out moves that would leave the player's king in check.
///
/// # Arguments
/// * `game` - The game state.
///
/// # Returns
/// * `Ok(ListOfMoves)` - All legal moves for the current player.
/// * `Err(Errors)` - If move generation fails.
pub fn generate_all_moves(game: &GameState) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    // Go through all squares
    for (location, piece_record) in game.piece_register.iter() {
        // If the piece belongs to the current turn
        if piece_record.team == game.turn {
            // Generate all the potential moves
            let potential_moves = match piece_record.class {
                crate::piece_types::PieceClass::Pawn => {
                    generate_potential_moves_pawn(game, &location)?
                }
                crate::piece_types::PieceClass::Knight => {
                    generate_potential_moves_knight(game, &location)?
                }
                crate::piece_types::PieceClass::Bishop => {
                    generate_potential_moves_bishop(game, &location)?
                }
                crate::piece_types::PieceClass::Rook => {
                    generate_potential_moves_rook(game, &location)?
                }
                crate::piece_types::PieceClass::Queen => {
                    generate_potential_moves_queen(game, &location)?
                }
                crate::piece_types::PieceClass::King => {
                    generate_potential_moves_king(game, &location)?
                }
            };
            // Make sure potential moves don't violate rules
            for k in potential_moves {
                let trial_game = apply_move_to_game(game, &k.description)?;
                // Verify that move did not allow a capture of king
                if can_capture_enemy_king(&trial_game)? {
                    // Can't do move that puts your king in check
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

    #[test]
    fn test_bishop_moves() -> Result<(), Errors> {
        let test_game = GameState::from_fen(
            "r2qk2r/1p1b1ppp/p1n1pn2/2b5/3P1B2/5N2/PPP1BPPP/R2QK2R w KQkq - 0 10",
        )
        .unwrap();
        let moves = generate_potential_moves_bishop(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 5);

        let test_game = GameState::from_fen(
            "r2qk2r/1p1b1ppp/p1n1pn2/2b5/3P1B2/5N2/PPP1BPPP/R2QK2R w KQkq - 0 10",
        )
        .unwrap();
        let moves = generate_potential_moves_bishop(&test_game, &(5, 3))?;
        assert_eq!(moves.len(), 10);

        Ok(())
    }

    #[test]
    fn test_rook_moves() -> Result<(), Errors> {
        let test_game =
            GameState::from_fen("4k2r/5ppp/p1nrp3/8/2R5/1P6/P4PPP/5R1K w k - 0 25").unwrap();
        let moves = generate_potential_moves_rook(&test_game, &(2, 3))?;
        assert_eq!(moves.len(), 12);

        Ok(())
    }

    #[test]
    fn test_queen_moves() -> Result<(), Errors> {
        let test_game =
            GameState::from_fen("r3k2r/1p1b1ppp/p1nBpn2/3q4/8/5N2/PPPQBPPP/R3K2R w KQkq - 2 13")
                .unwrap();
        let moves = generate_potential_moves_queen(&test_game, &(3, 1))?;
        assert_eq!(moves.len(), 12);

        Ok(())
    }

    #[test]
    fn test_king_moves() -> Result<(), Errors> {
        let test_game =
            GameState::from_fen("r3qrk1/pp3pb1/2pn1R1p/4P2Q/3p4/2NB3P/PPP3P1/R5K1 w - - 0 21")
                .unwrap();
        let moves = generate_potential_moves_king(&test_game, &(6, 0))?;
        assert_eq!(moves.len(), 4);
        
        // Cut-off from queen
        let test_game =
            GameState::from_fen("8/5k2/8/8/6K1/8/8/n3q3 w - - 12 147")
                .unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(), 6);
        Ok(())
    }

    #[test]
    fn test_check_moves() -> Result<(), Errors> {
        let test_game = GameState::from_fen("3k4/8/8/8/6b1/3pP3/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(), 4);

        Ok(())
    }

    #[test]
    fn test_back_row_stuff() -> Result<(), Errors> {
        let test_game =
            GameState::from_fen("1rb5/3k1p2/1p1P4/p3p1bP/P2n4/3RN2P/2R4K/1q3n2 w - - 8 53")
                .unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(), 4);

        let test_game =
            GameState::from_fen("3kn2Q/8/b7/p3p1P1/1r2P1p1/6p1/2BK4/8 b - - 6 90").unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(), 24);

        let test_game = GameState::from_fen("6kN/8/8/8/P7/8/1B5K/2b5 b - - 4 95").unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(), 8);

        Ok(())
    }

    #[test]
    fn test_promotion() -> Result<(), Errors> {
        // Simple Queen promotion
        let test_game = GameState::from_fen("8/P1k5/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let next_move = ChessMove {
            start: (0, 6),
            stop: (0, 7),
            move_specialness: MoveSpecialness::Promote(PieceClass::Queen),
        };
        let next_game = apply_move_to_game(&test_game, &next_move)?;
        let next_fen = next_game.get_fen();
        let desired_fen = String::from("Q7/2k5/8/8/8/8/8/4K3 b - - 0 1");
        assert_eq!(next_fen, desired_fen);

        // Knight check promotion into check
        let test_game = GameState::from_fen("7R/P1k5/7R/8/8/8/8/1Q1K4 w - - 0 1").unwrap();
        let next_move = ChessMove {
            start: (0, 6),
            stop: (0, 7),
            move_specialness: MoveSpecialness::Promote(PieceClass::Knight),
        };
        let next_game = apply_move_to_game(&test_game, &next_move)?;
        let next_fen = next_game.get_fen();
        let desired_fen = String::from("N6R/2k5/7R/8/8/8/8/1Q1K4 b - - 0 1");
        assert_eq!(next_fen, desired_fen);
        let next_moves = generate_all_moves(&next_game)?;        
        assert_eq!(next_moves.len(),1);

        Ok(())
    }

    #[test]
    fn test_castling_rights() -> Result<(), Errors> {
        // Simple case
        let test_game = GameState::from_fen("r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4").unwrap();
        let next_move = ChessMove {
            start: (4, 0),
            stop: (6, 0),
            move_specialness: MoveSpecialness::Castling(((7,0),(5,0))),
        };
        let next_game = apply_move_to_game(&test_game, &next_move)?;
        let next_fen = next_game.get_fen();
        let desired_fen = String::from("r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4");
        assert_eq!(next_fen, desired_fen);

        // Lost rights 1
        let test_game = GameState::from_fen("r1bqk2r/pppp1ppp/2n2n2/1B2p3/1b2P3/3P1N2/PPP2PPP/RNBQK2R w KQkq - 1 5").unwrap();
        let next_move = ChessMove {
            start: (4, 0),
            stop: (5, 0),
            move_specialness: MoveSpecialness::Regular,
        };
        let next_game = apply_move_to_game(&test_game, &next_move)?;
        let next_fen = next_game.get_fen();
        let desired_fen = String::from("r1bqk2r/pppp1ppp/2n2n2/1B2p3/1b2P3/3P1N2/PPP2PPP/RNBQ1K1R b kq - 2 5");
        assert_eq!(next_fen, desired_fen);

        // Lost rights 2
        let test_game = GameState::from_fen("r2Qkb1r/p1p2ppp/2p1bn2/4p3/4P3/2N2N2/PPP2PPP/R1B1K2R b KQkq - 0 8").unwrap();
        let next_move = ChessMove {
            start: (0, 7),
            stop: (3, 7),
            move_specialness: MoveSpecialness::Regular,
        };
        let next_game = apply_move_to_game(&test_game, &next_move)?;
        let next_fen = next_game.get_fen();
        let desired_fen = String::from("3rkb1r/p1p2ppp/2p1bn2/4p3/4P3/2N2N2/PPP2PPP/R1B1K2R w KQk - 0 9");
        assert_eq!(next_fen, desired_fen);


        Ok(())
    }


    #[test]
    fn test_castling_offer() -> Result<(), Errors> {
        // Simple castlings
        let mut test_game = GameState::from_fen("r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4").unwrap();
        let moves = generate_potential_moves_king(&test_game, &(4, 0))?;
        assert_eq!(moves.len(),3);

        // Execute castling
        let current_move = ChessMove::from_long_algebraic(&test_game,"e1g1")?;
        test_game = apply_move_to_game(&test_game, &current_move)?;
        assert_eq!(test_game.get_fen(),"r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4");

        // Execute capture
        let current_move = ChessMove::from_long_algebraic(&test_game,"f6e4")?;
        test_game = apply_move_to_game(&test_game, &current_move)?;
        assert_eq!(test_game.get_fen(),"r1bqkb1r/pppp1ppp/2n5/1B2p3/4n3/5N2/PPPP1PPP/RNBQ1RK1 w kq - 0 5");

        // No more castling available
        let moves = generate_potential_moves_king(&test_game, &(6, 0))?;
        assert_eq!(moves.len(),1);

        // All castling
        let mut test_game = GameState::from_fen("r3k2r/ppp1qppp/2np1n2/1Bb1p3/4P1b1/2NP1N2/PPPBQPPP/R3K2R w KQkq - 4 8").unwrap();
        let moves = generate_potential_moves_king(&test_game, &(4, 0))?;
        assert_eq!(moves.len(),4);

        // Execute castling
        let current_move = ChessMove::from_long_algebraic(&test_game,"e1c1")?;
        test_game = apply_move_to_game(&test_game, &current_move)?;
        assert_eq!(test_game.get_fen(),"r3k2r/ppp1qppp/2np1n2/1Bb1p3/4P1b1/2NP1N2/PPPBQPPP/2KR3R b kq - 5 8");

        // Execute castling
        let current_move = ChessMove::from_long_algebraic(&test_game,"e8g8")?;
        test_game = apply_move_to_game(&test_game, &current_move)?;
        assert_eq!(test_game.get_fen(),"r4rk1/ppp1qppp/2np1n2/1Bb1p3/4P1b1/2NP1N2/PPPBQPPP/2KR3R w - - 6 9");

        // No more castling
        let moves = generate_potential_moves_king(&test_game, &(2, 0))?;
        assert_eq!(moves.len(),1);

        // Blocked castling
        let test_game = GameState::from_fen("r3k2r/ppp1qppp/2Pp4/1B2p3/6b1/3P1N2/PbPBQPPP/R3K2R w KQkq - 0 11").unwrap();
        let moves = generate_potential_moves_king(&test_game, &(4, 0))?;
        assert_eq!(moves.len(),3);

        Ok(())
    }

    #[test]
    fn test_apply_lots_of_random_moves() -> Result<(),Errors>{

        // Has promotion
        let mut test_game = GameState::from_fen("r1bqk2r/pp2bppp/2np1n2/2p1p3/4P3/2P2NP1/PP1PQPBP/RNB2RK1 w kq - 1 8").unwrap();
        let moves_string = String::from("f1d1 b7b6 d2d4 c8e6 d4e5 a7a5 e5f6 b6b5 f6e7 a8a7 e7d8n e8d8 d1d6 d8c8 d6c6 c8b8 e2b5 a7b7 c1f4");        
        for token in moves_string.split_ascii_whitespace().into_iter(){
            let current_move = ChessMove::from_long_algebraic(&test_game,token)?;
            test_game = apply_move_to_game(&test_game, &current_move)?
        }
        let next_fen = test_game.get_fen();
        let desired_fen = String::from("1k5r/1r3ppp/2R1b3/pQp5/4PB2/2P2NP1/PP3PBP/RN4K1 b - - 2 17");
        assert_eq!(next_fen, desired_fen);
        

        // Has castling
        let mut test_game = GameState::from_fen("r1bqk2r/pp1n1ppp/2pbpn2/3p4/2PP4/2NBPN2/PPQ2PPP/R1B1K2R b KQkq - 5 7").unwrap();
        let moves_string = String::from("e8g8 c4c5 d6c5 c2d1 g7g6 d1e2 d8b6 g2g4 c5b4 a1b1 a7a6 e3e4 f8e8 e1d2 g8g7 d3b5 g7f8");
        for token in moves_string.split_ascii_whitespace().into_iter(){
            let current_move = ChessMove::from_long_algebraic(&test_game,token)?;
            test_game = apply_move_to_game(&test_game, &current_move)?
        }
        let next_fen = test_game.get_fen();
        let desired_fen = String::from("r1b1rk2/1p1n1p1p/pqp1pnp1/1B1p4/1b1PP1P1/2N2N2/PP1KQP1P/1RB4R w - - 5 16");
        assert_eq!(next_fen, desired_fen);

        // Has en passant
        let mut test_game = GameState::from_fen("r1bqkb1r/1p3ppp/2n1pn2/pBPp4/1P3B2/2P1P3/P4PPP/RN1QK1NR b KQkq - 0 7").unwrap();
        let moves_string = String::from("c8d7 d1e2 a5b4 f2f3 f8c5 a2a4 b4a3 g2g4 d8b6 b5a6 a8a6 b1a3 c5a3 e1d1 d5d4 e2b5 b6b5");
        for token in moves_string.split_ascii_whitespace().into_iter(){
            let current_move = ChessMove::from_long_algebraic(&test_game,token)?;
            test_game = apply_move_to_game(&test_game, &current_move)?;
        }
        let next_fen = test_game.get_fen();
        let desired_fen = String::from("4k2r/1p1b1ppp/r1n1pn2/1q6/3p1BP1/b1P1PP2/7P/R2K2NR w k - 0 16");
        assert_eq!(next_fen, desired_fen);

        Ok(())
    }

    #[test]
    fn test_en_passant() -> Result<(), Errors> {
        // Simple en passant
        let mut test_game = GameState::from_fen("rnbqkbnr/ppp1pppp/8/8/3pP1P1/7P/PPPP1P2/RNBQKBNR b KQkq e3 0 3").unwrap();
        let moves = generate_potential_moves_pawn(&test_game, &(3, 3))?;
        assert_eq!(moves.len(),2);

        // Move past another pawn
        let current_move = ChessMove::from_long_algebraic(&test_game,"f2f3")?;
        test_game = apply_move_to_game(&test_game, &current_move)?;

        // No more en passant will be available for this pawn
        let moves = generate_potential_moves_pawn(&test_game, &(3, 3))?;
        assert_eq!(moves.len(),1);

        // Complex en passant
        test_game = GameState::from_fen("r3kr2/p4p1p/p2q3p/P1pP4/5nn1/2p2bK1/8/7q w q c6 0 35").unwrap();
        let moves = generate_all_moves(&test_game)?;
        assert_eq!(moves.len(),1);
        for m in moves{
            dbg!(m);
        }
        
        Ok(())
    }




}
