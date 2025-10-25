use crate::{apply_move_to_game::apply_move_to_game_filtering_no_friendly_check, board_location::BoardLocation, board_mask::BoardMask, chess_errors::ChessErrors, collision_masks::{self, CollisionMasks}, game_state::GameState, generate_movements::{generate_bishop_movement, generate_king_movement, generate_knight_movement, generate_rook_movement}, generate_moves_level_3::GenerateLevel3Result, generate_moves_level_4::generate_moves_level_4, piece_record::PieceRecord, piece_register::{self, PieceRegister}, piece_team::PieceTeam, types_of_check::TypesOfCheck};

// Find all pieces causing check, or pinning down pieces
fn find_threatening_pieces(piece_register : &PieceRegister, king : &PieceRecord)-> Result<Vec<PieceRecord>,ChessErrors>{
    let mut threatening_pieces : Vec<PieceRecord> = vec![];
    let mut threat_mask : BoardMask = king.location.binary_location;
    for i in 0..8{
        if let Ok(position) = generate_king_movement(king.location,i){
            threat_mask |= position.binary_location;
        }
        if let Ok(position) = generate_knight_movement(king.location,i){
            threat_mask |= position.binary_location;
        }
    }
    for j in 0..4{
        for i in 0..8{
            if let Ok(position) = generate_bishop_movement(king.location,j,i){
                threat_mask |= position.binary_location;
            }
            if let Ok(position) = generate_rook_movement(king.location,j,i){
                threat_mask |= position.binary_location;
            }
        }
    }  
    let enemy_pieces = match king.team {
        crate::piece_team::PieceTeam::Light => &piece_register.dark_pieces,
        crate::piece_team::PieceTeam::Dark => &piece_register.light_pieces
    };
    for i in enemy_pieces{
        if i.0 & threat_mask > 0{
            threatening_pieces.push(*i.1);
        }
    }
    Ok(threatening_pieces)
}

fn sort_threats_to_pins_or_checks(collision_masks : &CollisionMasks, king : &PieceRecord, threatening_pieces : &Vec<PieceRecord>) -> Result<(Vec<PieceRecord>,Vec<PieceRecord>),ChessErrors>{

    let mut checking_pieces : Vec<PieceRecord> = vec![];
    let mut pinning_pieces : Vec<PieceRecord> = vec![];

    // Looking for threats on the friendly king location
    let friendly_king_mask = king.location.binary_location;

    // Look at all threatning piece moves
    for p in threatening_pieces{
        let generated_moves_level_3 = GenerateLevel3Result::from(p, &collision_masks)?;
        for c in generated_moves_level_3.captures{
            if c.binary_location & friendly_king_mask > 0{ // Someone is threatening the king
                checking_pieces.push(*p);
            }else{
                pinning_pieces.push(*p);
            }
        }
    }
    Ok((checking_pieces,pinning_pieces))
}

///  Inspects the game for check status
/// If last_piece_moved_optionis Some(piece), then we do a full check inspection and classification  
/// If it is None, we do a simple inspection without any check classification
pub fn inspect_check(game: &GameState, last_piece_moved_option : Option<PieceRecord>) -> Result<Option<TypesOfCheck>,ChessErrors>{
    
    // Look for threats via checks and pins
    let king = game.piece_register.view_king(game.turn)?;
    let threatening_pieces = find_threatening_pieces(&game.piece_register,king)?;
    let collision_masks = CollisionMasks::from(&game.piece_register);
    // Sort into checks or pins
    let (checking_pieces,_pinning_pieces) = sort_threats_to_pins_or_checks(&collision_masks, king, &threatening_pieces)?;
    // If nothing, answer now
    if checking_pieces.len() == 0{
        return Ok(None);
    }


    // Inspection for check / checkmate type

    if let Some(last_moved) = last_piece_moved_option{
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
    }else{
        Ok(Some(TypesOfCheck::UnclassifiedCheck(*checking_pieces.first().unwrap(), *king)))
    }
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_inspect_check(){
        let game = GameState::from_fen("rnb1kbnr/ppp1pppp/8/8/4P3/8/PPP2PPP/RNBqKBNR w KQkq - 0 4").unwrap();
        let check_inspection = inspect_check(&game, None).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::UnclassifiedCheck(_,_))));
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d1").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::SingleCheck(_,_))));

        let game = GameState::from_fen("Q4k2/7K/8/8/8/8/8/8 b - - 1 1").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::SingleCheck(_,_))));

        let game = GameState::from_fen("Q5k1/8/6K1/8/8/8/8/8 b - - 1 1").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::Checkmate(_,_))));

        let game = GameState::from_fen("Q4k2/3N4/8/6K1/8/8/8/8 b - - 3 2").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d7").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::DoubleCheck(_,_,_))));

        let game = GameState::from_fen("Q4k2/8/2N5/6K1/8/8/8/8 b - - 3 2").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("c6").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::DiscoveryCheck(_,_))));


    }
}