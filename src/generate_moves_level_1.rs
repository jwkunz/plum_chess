use std::collections::LinkedList;

use crate::{
    board_location::BoardLocation, chess_errors::ChessErrors, collision_masks::CollisionMasks, generate_movements::*, piece_team::PieceTeam
};

/*
Level 1 move generation is basic and only requires this information.
#[derive(Debug,Clone)]
pub struct GenerateLevel1Args{
    pub start: BoardLocation,
    pub masks: CollisionMasks,
}
*/

/// Type alias for a linked list of move descriptions with collision information.
pub type ListOfRawMoves = LinkedList<BoardLocation>;

/// The results from level 1 generation are moves on the board from a location until hitting the first collision
/// The collisions are sorted into types at this level
#[derive(Debug,Clone)]
pub struct GenerateLevel1Result{
    pub no_collisions : ListOfRawMoves,
    pub light_collisions : ListOfRawMoves,
    pub dark_collisions : ListOfRawMoves,
}

impl GenerateLevel1Result{
    /// Makes a new object
    pub fn new() -> Self{
        GenerateLevel1Result { no_collisions: ListOfRawMoves::new(), light_collisions: ListOfRawMoves::new(), dark_collisions: ListOfRawMoves::new()}
    }
    /// Sorts a raw moves
    pub fn sort_raw_move(&self, x : BoardLocation, masks : &CollisionMasks) -> Option<PieceTeam>{
        if x.binary_location&masks.light_mask > 0{
            return Some(PieceTeam::Light);
        }else if x.binary_location&masks.dark_mask > 0 {
            return Some(PieceTeam::Dark);
        }else{
            return None;
        }
    }
    /// Adds a raw move
    /// The bool indicates if there was collision found
    pub fn add_and_sort_raw_move(&mut self, x : BoardLocation, masks : &CollisionMasks) -> bool{
        self.add_sorted_moves(x, self.sort_raw_move(x, masks))
    }
    /// Adds a sorted raw move
    /// /// The bool indicates if there was collision found
    pub fn add_sorted_moves(&mut self, x : BoardLocation, sort : Option<PieceTeam>) -> bool{
        match sort{
            Some(PieceTeam::Light) => {
                self.light_collisions.push_back(x);
                return true;
            },
            Some(PieceTeam::Dark) => {
                self.dark_collisions.push_back(x);
                return true;
            },
            None => {
                self.no_collisions.push_back(x);
                return false;
            },
        }
    }
    /// Counts the number of moves found
    pub fn len(&self) -> usize{
        self.light_collisions.len() + self.dark_collisions.len() + self.no_collisions.len()
    }
}

/// The lowest level bishop moves
pub fn generate_bishop_moves_level_1(start : BoardLocation, masks : &CollisionMasks) -> Result<GenerateLevel1Result,ChessErrors>{
    let mut result = GenerateLevel1Result::new();
    // For all four directions
    for direction in 0..4{
        // For all distances
        'inner: for distance in 1..8{
            // Generate raw move
            if let Ok(x) = generate_bishop_movement(start,direction,distance){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
                // If it was a collision
                if sort.is_some(){
                    // Stop
                    break 'inner;
                }
            }else{
                break 'inner;
            }
        }
    }
    Ok(result)
}

/// The lowest level knight moves
pub fn generate_knight_moves_level_1(start : BoardLocation, masks : &CollisionMasks) -> Result<GenerateLevel1Result,ChessErrors>{
    let mut result = GenerateLevel1Result::new();
    // For all four directions
    for direction in 0..8{
            // Generate raw move
            if let Ok(x) = generate_knight_movement(start,direction){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
            }
        }
    Ok(result)
}
/// The lowest level rook moves
pub fn generate_rook_moves_level_1(start : BoardLocation, masks : &CollisionMasks) -> Result<GenerateLevel1Result,ChessErrors>{
    let mut result = GenerateLevel1Result::new();
    // For all four directions
    for direction in 0..4{
        // For all distances
        'inner: for distance in 1..8{
            // Generate raw move
            if let Ok(x) = generate_rook_movement(start,direction,distance){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
                // If it was a collision
                if sort.is_some(){
                    // Stop
                    break 'inner;
                }
            }else{
                break 'inner;
            }
        }
    }
    Ok(result)
}
/// The lowest level queen moves
pub fn generate_queen_moves_level_1(start : BoardLocation, masks : &CollisionMasks) -> Result<GenerateLevel1Result,ChessErrors>{
    let mut result = GenerateLevel1Result::new();
    // For all four directions
    for direction in 0..4{
        // For all distances
        'inner_rook: for distance in 1..8{
            // Generate raw move (Rook)
            if let Ok(x) = generate_rook_movement(start,direction,distance){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
                // If it was a collision
                if sort.is_some(){
                    // Stop
                    break 'inner_rook;
                }
            }else{
                break 'inner_rook;
            }
        }
        // For all distances
        'inner_bishop: for distance in 1..8{
            // Generate raw move (Bishop)
            if let Ok(x) = generate_bishop_movement(start,direction,distance){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
                // If it was a collision
                if sort.is_some(){
                    // Stop
                    break 'inner_bishop;
                }
            }else{
                break 'inner_bishop;
            }
        }
    }
    Ok(result)
}
/// The lowest level king moves
pub fn generate_king_moves_level_1(start : BoardLocation, masks : &CollisionMasks) -> Result<GenerateLevel1Result,ChessErrors>{
    let mut result = GenerateLevel1Result::new();
    // For all four directions
    for direction in 0..8{
            // Generate raw move
            if let Ok(x) = generate_king_movement(start,direction){
                // Sort it
                let sort = result.sort_raw_move(x, masks);
                // Add it
                result.add_sorted_moves(x,sort);
            }
        }
    Ok(result)
}


#[cfg(test)]
mod tests {
    use crate::game_state::GameState;

    use super::*;

    #[test]
    fn test_bishop_moves_level_1(){
        let game = GameState::from_fen("rnbqk1nr/p1p2ppp/4p3/3p4/1b1PP3/PpPB1N2/1P3PPP/RNBQK2R w KQkq - 1 7").unwrap();
        let raw_moves = generate_bishop_moves_level_1(BoardLocation::from_long_algebraic("d3").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),0);
        assert_eq!(raw_moves.light_collisions.len(),2);
        assert_eq!(raw_moves.no_collisions.len(),6);

        let raw_moves = generate_bishop_moves_level_1(BoardLocation::from_long_algebraic("b4").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),0);
        assert_eq!(raw_moves.light_collisions.len(),2);
        assert_eq!(raw_moves.no_collisions.len(),5);
    }

    #[test]
    fn test_knight_moves_level_1(){
        let game = GameState::from_fen("rnbqk1nr/p1p2ppp/4p3/3pN3/1b1PP3/PpPB4/1P3PPP/RNBQK2R b KQkq - 2 7").unwrap();
        let raw_moves = generate_knight_moves_level_1(BoardLocation::from_long_algebraic("e5").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),1);
        assert_eq!(raw_moves.light_collisions.len(),1);
        assert_eq!(raw_moves.no_collisions.len(),6);

        let raw_moves = generate_knight_moves_level_1(BoardLocation::from_long_algebraic("b8").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),0);
        assert_eq!(raw_moves.light_collisions.len(),0);
        assert_eq!(raw_moves.no_collisions.len(),3);
    }

    #[test]
    fn test_rook_moves_level_1(){
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let raw_moves = generate_rook_moves_level_1(BoardLocation::from_long_algebraic("a1").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),1);
        assert_eq!(raw_moves.light_collisions.len(),1);
        assert_eq!(raw_moves.no_collisions.len(),5);

        let raw_moves = generate_rook_moves_level_1(BoardLocation::from_long_algebraic("a8").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),2);
        assert_eq!(raw_moves.light_collisions.len(),0);
        assert_eq!(raw_moves.no_collisions.len(),0);
    }

    #[test]
    fn test_queen_moves_level_1(){
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let raw_moves = generate_queen_moves_level_1(BoardLocation::from_long_algebraic("d1").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),1);
        assert_eq!(raw_moves.light_collisions.len(),3);
        assert_eq!(raw_moves.no_collisions.len(),6);

        let raw_moves = generate_queen_moves_level_1(BoardLocation::from_long_algebraic("d8").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),4);
        assert_eq!(raw_moves.light_collisions.len(),0);
        assert_eq!(raw_moves.no_collisions.len(),6);
    }

    #[test]
    fn test_king_moves_level_1(){
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let raw_moves = generate_king_moves_level_1(BoardLocation::from_long_algebraic("e1").unwrap(),&CollisionMasks::from(&game.piece_register)).unwrap();
        assert_eq!(raw_moves.dark_collisions.len(),0);
        assert_eq!(raw_moves.light_collisions.len(),2);
        assert_eq!(raw_moves.no_collisions.len(),3);
    }
}