use crate::{board_location::BoardLocation, chess_errors::ChessErrors, game_state::GameState, piece_record::PieceRecord, piece_register::PieceRegister, piece_team::PieceTeam};

pub type MovementMask = u64;

/// Generates the binary mask for a pawn that is double stepping
/// 0 in mask means nothing was possible
/// Errors if piece_record is not a pawn
fn generate_pawn_double_step_movement_mask(piece_record : &PieceRecord, register : &PieceRegister)->Result<MovementMask,ChessErrors>{
    if !matches!(piece_record.class,crate::piece_class::PieceClass::Pawn){
        return Err(ChessErrors::GeneratingWrongMovementForPieceType(piece_record.class));
    } 
    let (start_file,start_rank) = piece_record.location.get_file_rank();
    if (start_rank != 1 && matches!(piece_record.team,PieceTeam::Light)) || (start_rank != 6 && matches!(piece_record.team,PieceTeam::Dark)){
        return Ok(0);
    }
    let direction : i8 = match piece_record.team {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    };
    let mut result : MovementMask = piece_record.location.binary_location;
    for i in 1..=2{
        let next_rank = start_rank as i8 + direction*i;
        let next_location = BoardLocation::from_file_rank(start_file, next_rank as u8)?;
        if register.view_piece_at_location(next_location).is_ok(){
            return Ok(0);
        }
        result |= next_location.binary_location;
    }
    Ok(result)
}

/// Generates the binary mask for a pawn that is single stepping
/// 0 in mask means nothing was possible
/// Errors if piece_record is not a pawn
fn generate_pawn_single_step_movement_mask(piece_record : &PieceRecord, register : &PieceRegister)->Result<MovementMask,ChessErrors>{
    if !matches!(piece_record.class,crate::piece_class::PieceClass::Pawn){
        return Err(ChessErrors::GeneratingWrongMovementForPieceType(piece_record.class));
    } 
    let (start_file,start_rank) = piece_record.location.get_file_rank();
    let direction : i8 = match piece_record.team {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    };
    let mut result : MovementMask = piece_record.location.binary_location;
    
    if let Ok(next_location) = BoardLocation::generate_moved_location_checked(&piece_record.location, 0, direction){
        if register.view_piece_at_location(next_location).is_ok(){
            return Ok(0);
        }else{
            result |= next_location.binary_location;
        }
    }
    Ok(result)
}

/// Generates the binary mask for a pawn that is single stepping
/// 0 in mask means nothing was possible
/// Errors if piece_record is not a pawn
fn generate_pawn_diagonal_captures_mask(piece_record : &PieceRecord, register : &PieceRegister)->Result<MovementMask,ChessErrors>{
    if !matches!(piece_record.class,crate::piece_class::PieceClass::Pawn){
        return Err(ChessErrors::GeneratingWrongMovementForPieceType(piece_record.class));
    } 
    let direction : i8 = match piece_record.team {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    };
    let mut result : MovementMask = 0;
    
    if let Ok(next_location) = BoardLocation::generate_moved_location_checked(&piece_record.location, 1, direction){
        if register.view_piece_at_location(next_location).is_ok(){
            result |= next_location.binary_location;
        }
    }
    if let Ok(next_location) = BoardLocation::generate_moved_location_checked(&piece_record.location, -1, direction){
        if register.view_piece_at_location(next_location).is_ok(){
            result |= next_location.binary_location;
        }
    }
    if result > 0{
        result |= piece_record.location.binary_location;
    }
    Ok(result)
}

pub fn generate_available_movement_mask(piece :&PieceRecord, game: &GameState)->Result<MovementMask,ChessErrors>{
    let (start_file,start_rank) = piece.location.get_file_rank();
    let result = match piece.class {
        crate::piece_class::PieceClass::Pawn => 0,
        crate::piece_class::PieceClass::Knight => 0,
        crate::piece_class::PieceClass::Bishop => 0,
        crate::piece_class::PieceClass::Rook => 0,
        crate::piece_class::PieceClass::Queen => 0,
        crate::piece_class::PieceClass::King => 0
    };
    
    Ok(result)
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_generate_pawn_double_step_movement_mask(){
        let game = GameState::new_game();
        let mask = generate_pawn_double_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e2").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e2").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e3").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e4").unwrap().binary_location;
        assert_eq!(mask,expected);

        let mask = generate_pawn_double_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e7").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e7").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e6").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e5").unwrap().binary_location;
        assert_eq!(mask,expected);

        let game = GameState::from_fen("rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2").unwrap();
        let mask = generate_pawn_double_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e4").unwrap()).unwrap(),&game.piece_register).unwrap();
        let expected = 0;
        assert_eq!(mask,expected);
    }
    #[test]
    fn test_generate_pawn_single_step_movement_mask(){
        let game = GameState::new_game();
        let mask = generate_pawn_single_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e2").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e2").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e3").unwrap().binary_location;
        assert_eq!(mask,expected);

        let mask = generate_pawn_single_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e7").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e7").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e6").unwrap().binary_location;
        assert_eq!(mask,expected);

        let game = GameState::from_fen("rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2").unwrap();
        let mask = generate_pawn_single_step_movement_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e4").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e4").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e5").unwrap().binary_location;
        assert_eq!(mask,expected);
    }    
    #[test]
    fn test_generate_pawn_capture_movement_mask(){
        let game = GameState::from_fen("rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let mask = generate_pawn_diagonal_captures_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e4").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("e4").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("d5").unwrap().binary_location;
        assert_eq!(mask,expected);

        let mask = generate_pawn_diagonal_captures_mask(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d5").unwrap()).unwrap(),&game.piece_register).unwrap();
        let mut expected = BoardLocation::from_long_algebraic("d5").unwrap().binary_location;
        expected |= BoardLocation::from_long_algebraic("e4").unwrap().binary_location;
        assert_eq!(mask,expected);
    }       
}