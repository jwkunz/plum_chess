use std::collections::LinkedList;

use crate::{
    board_location::{move_board_location, BoardLocation},
    chess_move::{ChessMove, MoveSpecialness},
    errors::Errors,
    game_state::GameState,
    piece_types::{PieceClass, PieceTeam},
};

#[derive(Clone, Debug)]
pub enum Occupancy {
    Empty,        // Empty square
    EnemyRegular, // Non-king occupied square of other side
    EnemyKing,    // King occupied square of other side
}

#[derive(Clone, Debug)]
pub struct ChessMoveDescriptionWithCollision {
    pub description: ChessMove,
    pub stop_occupancy: Occupancy,
}

type ListOfMoves = LinkedList<ChessMoveDescriptionWithCollision>;

/// This function will apply the chess_move to a game to create a new game state
pub fn apply_move_to_game(game: &GameState, chess_move: &ChessMove) -> Result<GameState, Errors> {
    let mut result = game.clone();
    if let Some(mut piece) = result.piece_register.remove_piece_record(&chess_move.start) {
        // Update counters and turn
        if matches!(piece.class,PieceClass::Pawn){
            result.half_move_clock = 0;
        }
        if matches!(piece.team,PieceTeam::Dark){
            result.full_move_count += 1;
            result.turn = PieceTeam::Light;
        }else{
            result.turn = PieceTeam::Dark;
        }

        // Do move
        match chess_move.move_specialness {
            MoveSpecialness::Regular => {
                result
                    .piece_register
                    .add_piece_record_overwrite(piece, &chess_move.stop)?;
            }
            MoveSpecialness::Castling((rook_start, rook_stop)) => {}
            MoveSpecialness::EnPassant(behind_pawn) => {}
            MoveSpecialness::Promote(target_type) => {
                    piece.class = target_type;
                    result
                        .piece_register
                        .add_piece_record_overwrite(piece, &chess_move.stop)?;
                } 
            }

    } else {
        return Err(Errors::InvalidMoveStartCondition);
    };

    Ok(result)
}

// This function checks if a given game state allows capturing an enemy king in the given turn
// Depending on whose turn is active this can be used to inspect for "check"
fn can_capture_enemy_king(game: &GameState) -> Result<bool, Errors> {
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
            // Look for a king collision
            for k in potential_moves {
                if matches!(k.stop_occupancy, Occupancy::EnemyKing) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

// Returns the vector for forward depending on whose turn it is
fn get_forward_direction_for_turn(turn: &PieceTeam) -> i8 {
    match turn {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    }
}

// Checks if a location is given piece type
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

        // Check for en passant move
        let move_specialness = if (stop.1 - start.1).abs() == 2 {
            // Is a double step
            let en_passant_square = (start.0, (start.1 + stop.1) / 2); // Right behind
            if game.piece_register.view(&en_passant_square).is_some() {
                // Can't jump over a piece in the en passant spot
                return false;
            }
            MoveSpecialness::EnPassant(en_passant_square)
        } else {
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

// Generates all possible move before evaluating for check
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
    // Return whatever was available
    Ok(result)
}

// Considers start and stop as a potential move
// and if feasible based on collision
// Does not inspect rules for check
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

// Helper for follow move vector until edge of board or enemy collision
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
    // Try castling

    Ok(result)
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

    // TODO
    // Need to test castling (create, explore, and execute)
    // Need to test en passant (create, exploration, and execute)
}
