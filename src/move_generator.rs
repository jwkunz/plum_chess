use std::collections::LinkedList;

use crate::{
    game_state::{self, GameState},
    types::{BoardLocation, Class},
};

pub enum MoveType {
    Movement,
    Promote(Class),           // Type to promote
    EnPassant(BoardLocation), // Behind pawn,
    Castling(BoardLocation),  // For rook,
}

pub enum CollisionType {
    NoCollision,
    Capture,
    EndOfBoard,
}

pub struct ChessMoveDescription {
    pub start: BoardLocation,
    pub stop: BoardLocation,
    pub move_type: MoveType,
    pub collision_type: CollisionType,
    pub caused_check: bool,
}

type ListOfMoves = LinkedList<ChessMoveDescription>;

pub fn does_move_create_check(game: &GameState, candidate_move: &ChessMoveDescription) -> bool {
    true
}

pub fn generate_pawn_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}
pub fn generate_knight_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}
pub fn generate_bishop_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}
pub fn generate_rook_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}
pub fn generate_queen_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}
pub fn generate_king_moves_till_collide(game: &GameState, start: &BoardLocation) -> ListOfMoves {
    LinkedList::new()
}

pub fn generate_all_moves(game: &GameState) -> ListOfMoves {
    let mut result = LinkedList::new();
    for (location, piece_record) in game.piece_register.iter() {
        if piece_record.affiliation == game.turn {
            let moves_till_collide = match piece_record.class {
                crate::types::Class::Pawn => generate_pawn_moves_till_collide(game, &location),
                crate::types::Class::Knight => generate_knight_moves_till_collide(game, &location),
                crate::types::Class::Bishop => generate_bishop_moves_till_collide(game, &location),
                crate::types::Class::Rook => generate_rook_moves_till_collide(game, &location),
                crate::types::Class::Queen => generate_queen_moves_till_collide(game, &location),
                crate::types::Class::King => generate_king_moves_till_collide(game, &location),
            };
            for k in moves_till_collide {
                if !does_move_create_check(game, &k) {
                    result.push_back(k);
                }
            }
        }
    }
    result
}
