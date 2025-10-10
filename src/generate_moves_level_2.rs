use crate::{
    board_location::BoardLocation,chess_errors::ChessErrors, generate_movements::*, generate_moves_level_1::*, piece_team::PieceTeam
};

/*
Level 2 movement only need this information:

#[derive(Debug,Clone)]
pub struct GenerateLevel2Args{
    pub start: BoardLocation,
    pub masks: CollisionMasks,
    pub team: PieceTeam
}

impl GenerateLevel2Args {
    pub fn from(start : BoardLocation,piece_register : &PieceRegister, team : PieceTeam)->Self{
        GenerateLevel2Args { start, masks: CollisionMasks::from(piece_register, team)}
    }
}
*/


/// The results from level 2 generation are level 1 generations filtered into captures / regular moves
#[derive(Debug,Clone)]
pub struct GenerateLevel2Result{
    pub no_collisions : ListOfRawMoves,
    pub captures : ListOfRawMoves,
}
impl GenerateLevel2Result{
    /// Makes a new object
    pub fn new() -> Self{
        GenerateLevel2Result { no_collisions: ListOfRawMoves::new(), captures: ListOfRawMoves::new()}
    }
    /// Filter level 1 results by team
    pub fn from(x : GenerateLevel1Result, team : PieceTeam) -> Self{
        GenerateLevel2Result{
            no_collisions : x.no_collisions,
            captures : match team {
                PieceTeam::Dark => x.light_collisions,
                PieceTeam::Light => x.dark_collisions
            }
        }
    }

    /// Counts the number of moves found
    pub fn len(&self) -> usize{
        self.captures.len() + self.no_collisions.len()
    }
}

/// Pawns require level 2 information to generate
pub fn generate_pawn_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let mut result_1 = GenerateLevel1Result::new();

    // Check first movement
    if let Ok(x) = generate_pawn_single_step_movement(start,team){
        if result_1.add_and_sort_raw_move(x,masks) == false{
            // Check second movement if first was not a collision
            let (_,start_file) = start.get_file_rank();
            // If on starting point
            if ((start_file == 1) && matches!(team,PieceTeam::Light)) || ((start_file == 6) && matches!(team,PieceTeam::Dark)){
                    if let Ok(x) = generate_pawn_double_step_movement(start,team){
                        result_1.add_and_sort_raw_move(x,masks);
                }
            }
        }
    }
    
    // Check left capture
    if let Ok(x) = generate_pawn_capture_movement(start,team,-1){
        // Is a collision
        if let Some(y) = result_1.sort_raw_move(x,masks){
            // Of the other team
            if y != team{
                result_1.add_sorted_moves(x,Some(y));
            }
        }
    }
    // Check right capture
    if let Ok(x) = generate_pawn_capture_movement(start,team,1){
        // Is a collision
        if let Some(y) = result_1.sort_raw_move(x,masks){
            // Of the other team
            if y != team{
                result_1.add_sorted_moves(x,Some(y));
            }
        }
    } 

    Ok(GenerateLevel2Result::from(result_1,team))
}

/// The second level knight moves (filter pass through)
pub fn generate_knight_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let result_1 = generate_knight_moves_level_1(start, masks)?;
    Ok(GenerateLevel2Result::from(result_1,team))
}

/// The second level bishop moves (filter pass through)
pub fn generate_bishop_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let result_1 = generate_bishop_moves_level_1(start, masks)?;
    Ok(GenerateLevel2Result::from(result_1,team))
}

/// The second level rook moves (filter pass through)
pub fn generate_rook_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let result_1 = generate_rook_moves_level_1(start, masks)?;
    Ok(GenerateLevel2Result::from(result_1,team))
}

/// The second level queen moves (filter pass through)
pub fn generate_queen_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let result_1 = generate_queen_moves_level_1(start, masks)?;
    Ok(GenerateLevel2Result::from(result_1,team))
}

/// The second level king moves (filter pass through)
pub fn generate_king_moves_level_2(start : BoardLocation, masks : &CollisionMasks, team:PieceTeam) -> Result<GenerateLevel2Result,ChessErrors>{
    let result_1 = generate_king_moves_level_1(start, masks)?;
    Ok(GenerateLevel2Result::from(result_1,team))
}

#[cfg(test)]
mod tests {
    use crate::game_state::GameState;

    use super::*;

    #[test]
    fn test_pawn_moves_level_2(){
        let game = GameState::from_fen("rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let raw_moves = generate_pawn_moves_level_2(BoardLocation::from_long_algebraic("e4").unwrap(),&CollisionMasks::from(&game.piece_register),PieceTeam::Light).unwrap();
        assert_eq!(raw_moves.len(),2);

        let raw_moves = generate_pawn_moves_level_2(BoardLocation::from_long_algebraic("d5").unwrap(),&CollisionMasks::from(&game.piece_register),PieceTeam::Dark).unwrap();
        assert_eq!(raw_moves.len(),2);

        let game = GameState::from_fen("rnbqkbnr/p1p2ppp/4p3/3p4/3PP3/Pp1B1N2/1PP2PPP/RNBQK2R w KQkq - 0 6").unwrap();
        let raw_moves = generate_pawn_moves_level_2(BoardLocation::from_long_algebraic("c2").unwrap(),&CollisionMasks::from(&game.piece_register),PieceTeam::Light).unwrap();
        assert_eq!(raw_moves.captures.len(),1);
        assert_eq!(raw_moves.no_collisions.len(),2);
    }
}