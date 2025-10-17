use crate::{chess_errors::ChessErrors, collision_masks::CollisionMasks, game_state::GameState, generate_moves_level_3::{GenerateLevel3Result}};

pub fn inspect_if_game_has_king_in_check(game : &GameState) -> Result<bool,ChessErrors>{
    let result = false;
    let collision_masks = CollisionMasks::from(&game.piece_register);
    if matches!(game.turn,crate::piece_team::PieceTeam::Light){
        // Looking for threats on the light king location
        let king_mask = game.piece_register.generate_mask_light_king()?;
        // Look at all dark piece moves
        for (_,p) in &game.piece_register.dark_pieces{
            let generated_moves_level_3 = GenerateLevel3Result::from(p, &collision_masks)?;
            for c in generated_moves_level_3.captures{
                if c.binary_location & king_mask > 0{ // Someone is threatening the king
                    return Ok(true)
                }
            }
        }
    }else{
        // Looking for threats on the dark king location
        let king_mask = game.piece_register.generate_mask_dark_king()?;
        // Look at all light piece moves
        for (_,p) in &game.piece_register.light_pieces{
            let generated_moves_level_3 = GenerateLevel3Result::from(p, &collision_masks)?;
            for c in generated_moves_level_3.captures{
                if c.binary_location & king_mask > 0{ // Someone is threatening the king
                    return Ok(true)
                }
            }
        }
    }
    Ok(result)
}