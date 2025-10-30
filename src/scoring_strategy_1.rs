use crate::{chess_errors::ChessErrors, game_state::GameState, scoring::{CanScoreGame, Score, alpha_zero_score}};

/*
use crate::{collision_masks::CollisionMasks, generate_moves_level_3::GenerateLevel3Result};

fn count_material_in_center(game:&GameState) -> Score{
    let mut result : Score = 0.0;
    let mut total_white : Score = 0.0;
    let mut total_dark : Score = 0.0;
    for (_,value) in &game.piece_register.light_pieces{
        let (file,rank) = value.location.get_file_rank();
        if file > 1 && file < 6 && rank > 1 && rank < 6{
            result += 1.0;
            total_white += 1.0;
        }
    }
    for (_,value) in &game.piece_register.dark_pieces{
        let (file,rank) = value.location.get_file_rank();
        if file > 1 && file < 6 && rank > 1 && rank < 6{
            result -= 1.0;
            total_dark += 1.0;
        }
    }    
    let max = if total_white > total_dark {
        total_white
    }else{
        total_dark
    };
    3.0 * result / max
}

fn count_attacking_squares(game:&GameState) -> Result<Score,ChessErrors>{
    let mut result : Score = 0.0;
    let masks = CollisionMasks::from(&game.piece_register);
    let mut total_white : Score = 0.0;
    let mut total_dark : Score = 0.0;
    for (_,value) in &game.piece_register.light_pieces{
        let moves = GenerateLevel3Result::from(value,&masks)?;
        result += 2.0*moves.captures.len() as Score;
        result += 1.0*moves.no_collisions.len() as Score;
        total_white += 2.0*moves.captures.len() as Score;
        total_white += 1.0*moves.no_collisions.len() as Score;
    }
    for (_,value) in &game.piece_register.dark_pieces{
        let moves = GenerateLevel3Result::from(value,&masks)?;
        result -= 2.0*moves.captures.len() as Score;
        result -= 1.0*moves.no_collisions.len() as Score;
        total_dark += 2.0*moves.captures.len() as Score;
        total_dark += 1.0*moves.no_collisions.len() as Score;
    }  
    let max = if total_white > total_dark {
        total_white
    }else{
        total_dark
    };
    Ok(3.0 * result / max)
}
*/

fn calculate_alpha_zero_score(game:&GameState) -> Result<Score,ChessErrors>{
    let mut result : Score = 0.0;
    for (_,value) in &game.piece_register.light_pieces{
        result += alpha_zero_score(&value.class);
    }
    for (_,value) in &game.piece_register.dark_pieces{
        result -= alpha_zero_score(&value.class);
    }  
    Ok(result)
}


/// The simplest scoring object
pub struct ScoringStrategy1{}
impl CanScoreGame for ScoringStrategy1{
    fn calculate_score(game : &GameState) -> Result<Score,ChessErrors> {
        calculate_alpha_zero_score(game)
    }
    fn new() -> Self{
        ScoringStrategy1 {  }
    }
}