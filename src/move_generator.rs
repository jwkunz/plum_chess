use std::{collections::LinkedList, io::ErrorKind};

use crate::{
    errors::Errors,
    game_state::{self, GameState},
    types::{move_board_location, Affiliation, BoardLocation, Class},
};

#[derive(Clone, Debug)]
pub enum MoveSpecialness {
    Regular,                  // Used for moving and capturing
    Promote(Class),           // Class is type to promote
    EnPassant(BoardLocation), // BoardLocation is behind pawn,
    Castling(BoardLocation),  // Board location is for rook,
}

#[derive(Clone, Debug)]
pub enum CollisionType {
    NoCollision,           // Empty square
    OwnSidePiece,          // An own side piece collision
    OtherSideRegularPiece, // Non-king occupied square of other side
    OtherSideKingPiece,    // King occupied square of other side
}

#[derive(Clone, Debug)]
pub struct ChessMoveDescription {
    pub start: BoardLocation,
    pub stop: BoardLocation,
    pub move_type: MoveSpecialness,
    pub collision_type: CollisionType,
}

type ListOfMoves = LinkedList<ChessMoveDescription>;

pub fn does_move_create_check(game: &GameState, candidate_move: &ChessMoveDescription) -> bool {
    true
}

fn get_forward_direction(game: &GameState) -> i8 {
    match game.turn {
        Affiliation::Dark => -1,
        Affiliation::Light => 1,
    }
}

// Common pawn move actions associated with a candidate move
fn pawn_move_helper(
    game: &GameState,
    start: &BoardLocation,
    stop: &BoardLocation,
    result: &mut ListOfMoves,
) {
    // Assume no collision occurs
    let mut collision_type = CollisionType::NoCollision;
    // Unless
    if let Some(target) = game.piece_register.view(stop) {
        // Something was at the stop location

        // Look if the move was forward
        if start.0 == stop.0 {
            // Cannot capture moving forward
            return;
        }

        // What kind of piece collision was it?
        collision_type = if game.turn == target.affiliation {
            //CollisionType::OwnSidePiece
            return;
        } else {
            match target.class {
                Class::King => CollisionType::OtherSideKingPiece,
                _ => CollisionType::OtherSideRegularPiece,
            }
        };
    } else {
        // Look if the move was diagonal
        if start.0 != stop.0 {
            // Cannot move diagonal without capture
            return;
        }
    }
    // Look for promotion on back rank
    if (game.turn == Affiliation::Light && stop.1 == 7)
        || (game.turn == Affiliation::Dark && stop.1 == 0)
    {
        // All the kinds of promotions
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_type: MoveSpecialness::Promote(Class::Queen),
            collision_type: collision_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_type: MoveSpecialness::Promote(Class::Rook),
            collision_type: collision_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_type: MoveSpecialness::Promote(Class::Bishop),
            collision_type: collision_type.clone(),
        });
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_type: MoveSpecialness::Promote(Class::Knight),
            collision_type: collision_type.clone(),
        });
    } else {
        // A regular move (capture or movement)
        let move_type = if (stop.1 - start.1).abs() == 2 {
            let en_passant_square = (start.0, (start.1 + stop.1) / 2);
            if game.piece_register.view(&en_passant_square).is_some() {
                // Can't jump over a piece in the en passant spot
                return;
            }
            MoveSpecialness::EnPassant(en_passant_square)
        } else {
            MoveSpecialness::Regular
        };
        result.push_back(ChessMoveDescription {
            start: *start,
            stop: *stop,
            move_type,
            collision_type: collision_type.clone(),
        });
    };
}

pub fn generate_pawn_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    if let Some(x) = game.piece_register.view(start) {
        match x.class {
            Class::Pawn => (),
            _ => return Err(Errors::MoveStartLocationIsNotValidPieceType),
        }
    } else {
        return Err(Errors::MoveStartLocationIsNotValidPieceType);
    }

    let forward_direction = get_forward_direction(game);
    if let Ok(stop) = move_board_location(start, 1, forward_direction) {
        pawn_move_helper(game, start, &stop, &mut result);
    }
    if let Ok(stop) = move_board_location(start, -1, forward_direction) {
        pawn_move_helper(game, start, &stop, &mut result);
    }
    if let Ok(stop) = move_board_location(start, 0, forward_direction) {
        pawn_move_helper(game, start, &stop, &mut result);
    }
    let start_square = match game.turn {
        Affiliation::Dark => 6,
        Affiliation::Light => 1,
    };
    if start_square == start.1 {
        if let Ok(stop) = move_board_location(start, 0, 2 * forward_direction) {
            pawn_move_helper(game, start, &stop, &mut result);
        }
    }
    Ok(result)
}
pub fn generate_knight_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_bishop_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_rook_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_queen_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}
pub fn generate_king_moves_till_collide(
    game: &GameState,
    start: &BoardLocation,
) -> Result<ListOfMoves, Errors> {
    Ok(LinkedList::new())
}

pub fn generate_all_moves(game: &GameState) -> Result<ListOfMoves, Errors> {
    let mut result = LinkedList::new();
    for (location, piece_record) in game.piece_register.iter() {
        if piece_record.affiliation == game.turn {
            let moves_till_collide = match piece_record.class {
                crate::types::Class::Pawn => generate_pawn_moves_till_collide(game, &location)?,
                crate::types::Class::Knight => generate_knight_moves_till_collide(game, &location)?,
                crate::types::Class::Bishop => generate_bishop_moves_till_collide(game, &location)?,
                crate::types::Class::Rook => generate_rook_moves_till_collide(game, &location)?,
                crate::types::Class::Queen => generate_queen_moves_till_collide(game, &location)?,
                crate::types::Class::King => generate_king_moves_till_collide(game, &location)?,
            };
            for k in moves_till_collide {
                if !does_move_create_check(game, &k) {
                    result.push_back(k);
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pawn_moves() -> Result<(), Errors> {
        let test_game = GameState::from_fen("3k4/8/8/8/8/8/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_pawn_moves_till_collide(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 2);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3p4/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_pawn_moves_till_collide(&test_game, &(4, 1))?;
        assert_eq!(moves.len(), 3);

        let test_game = GameState::from_fen("3k4/4P3/8/8/8/8/8/3K4 w - - 0 1").unwrap();
        let moves = generate_pawn_moves_till_collide(&test_game, &(4, 6))?;
        assert_eq!(moves.len(), 8);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3pP3/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_pawn_moves_till_collide(&test_game, &(4, 1))?;
        dbg!(moves.clone());
        assert_eq!(moves.len(), 1);

        let test_game = GameState::from_fen("3k4/8/8/8/8/3pP3/4P3/3K4 w - - 0 1").unwrap();
        let moves = generate_pawn_moves_till_collide(&test_game, &(4, 2))?;
        dbg!(moves.clone());
        assert_eq!(moves.len(), 1);

        Ok(())
    }
}
