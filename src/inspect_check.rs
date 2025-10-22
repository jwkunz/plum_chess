use crate::{apply_move_to_game::apply_move_to_game_filtering_no_friendly_check, chess_errors::ChessErrors, collision_masks::CollisionMasks, game_state::GameState, generate_moves_level_3::GenerateLevel3Result, generate_moves_level_4::generate_moves_level_4, piece_record::PieceRecord, types_of_check::TypesOfCheck};

///  Inspects the game for check status
/// If last_piece_moved_optionis Some(piece), then we do a full check inspection and classification  
/// If it is None, we do a simple inspection without any check classification
pub fn inspect_check(game: &GameState, last_piece_moved_option : Option<PieceRecord>) -> Result<Option<TypesOfCheck>,ChessErrors>{
    // See how many pieces are checking the king
    let collision_masks = CollisionMasks::from(&game.piece_register);
    let mut checking_pieces : Vec<PieceRecord> = vec![];
    let enemy_turn = match game.turn {
        crate::piece_team::PieceTeam::Light => crate::piece_team::PieceTeam::Dark,
        crate::piece_team::PieceTeam::Dark => crate::piece_team::PieceTeam::Light
    };
    // Looking for threats on the friendly king location
    let friendly_king_mask = match game.turn {
        crate::piece_team::PieceTeam::Light => game.piece_register.generate_mask_light_king()?,
        crate::piece_team::PieceTeam::Dark => game.piece_register.generate_mask_dark_king()?
    };
    let enemy_pieces = match enemy_turn {
        crate::piece_team::PieceTeam::Light => &game.piece_register.light_pieces,
        crate::piece_team::PieceTeam::Dark => &game.piece_register.dark_pieces
    };

    // Look at all enemy piece moves
    for (_,p) in enemy_pieces{
        let generated_moves_level_3 = GenerateLevel3Result::from(p, &collision_masks)?;
        for c in generated_moves_level_3.captures{
            if c.binary_location & friendly_king_mask > 0{ // Someone is threatening the king
                if last_piece_moved_option.is_none(){
                    return Ok(Some(TypesOfCheck::UnclassifiedCheck(*game.piece_register.view_king(enemy_turn)?, *p)));
                }else{
                    checking_pieces.push(*p);
                }
            }
        }
    }
    // No further check testing needed
    if checking_pieces.len() == 0{
        return Ok(None);
    }
    // Unwrape the last piece moved because it must be some
    let last_moved = last_piece_moved_option.expect("Last piece moved must be Some to get here");

    // Inspection for checkmate

    // Look if friendly moves can get out of check  
    let friendly_pieces = match game.turn {
        crate::piece_team::PieceTeam::Light => &game.piece_register.light_pieces,
        crate::piece_team::PieceTeam::Dark => &game.piece_register.dark_pieces
    };
    for (_,p) in friendly_pieces {  
        let generated_moves_level_4 = generate_moves_level_4(
            p,
            &collision_masks,
            &game.special_flags,
            &game.piece_register
        )?;
        // For each move
        for move_to_try in generated_moves_level_4 {
            //dbg!(format!("Move: {:?} in {:?}",move_to_try, game.get_fen()));
            // Simulate the future game, and make sure it doesn't create friendly check
            if let Some(_) = apply_move_to_game_filtering_no_friendly_check(&move_to_try, game)?
            {
                // Found a move that gets out of check
                return match checking_pieces.len(){
                    1 => { // One piece is checking, figure out if it's a discovery check based on the last piece moved and checking piece
                        if checking_pieces[0].location.binary_location == last_moved.location.binary_location{
                            Ok(Some(TypesOfCheck::SingleCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone())))
                        } else {
                            Ok(Some(TypesOfCheck::DiscoveryCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone())))
                        }
                    },
                    2 => Ok(Some(TypesOfCheck::DoubleCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone(),checking_pieces[1].clone()))),
                    _ => Err(ChessErrors::ErrorDuringCheckInspection("More than two checking pieces found when classifying check type".to_string()))
                };
                
            }
        }
    }
    // No moves found to get out of check, so it's checkmate
    return Ok(Some(TypesOfCheck::Checkmate(*game.piece_register.view_king(game.turn)?, checking_pieces[0].clone())));
}