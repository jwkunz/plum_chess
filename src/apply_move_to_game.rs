use crate::{chess_errors::ChessErrors, game_state::GameState, inspect_if_king_is_under_check::inspect_if_game_has_king_in_check, move_description::{MoveDescription, MoveTypes}, piece_class::PieceClass, piece_team::PieceTeam};

/// Applies an unchecked chess move to a given game state, returning the resulting game state or an error.
/// This function handles all move types, including castling, en passant, promotion, and updates castling rights and clocks.
/// It will not update the game's check status
/// 
/// # Arguments
/// * `chess_move` - The move to apply.
/// * `game` - The current game state.
///
/// # Returns
/// * `Ok(GameState)` - The new game state after the move.
/// * `Err(Errors)` - If the move is invalid or cannot be applied.
pub fn apply_move_to_game_unchecked(chess_move: &MoveDescription, game: &GameState) -> Result<GameState, ChessErrors> {
    let mut result = game.clone();
    let mut remove_castling_kingside_rights = false;
    let mut remove_castling_queenside_rights = false;
    let mut capture_flag = false;
    let moving_a_pawn = matches!(chess_move.vector.piece_at_start.class,PieceClass::Pawn); 

    // Handle the move based on its specialness (regular, castling, promotion, etc.)
    match chess_move.move_type {
        MoveTypes::Regular => {

            // Move the piece, possibly capturing an enemy piece

            // Handle capture
            if let Some(x) = chess_move.capture_status{
                capture_flag = true;
                result.piece_register.remove_piece_at_location(x.location)?;
            }

            // Handle movement
            let future_piece = result.piece_register.edit_piece_at_location(chess_move.vector.piece_at_start.location)?;
            future_piece.location = chess_move.vector.destination;
            
            // Remove castling rights if a king or rook moves.
            if matches!(future_piece.class,PieceClass::King){
                remove_castling_kingside_rights = true;
                remove_castling_queenside_rights = true;
            }
            // Flag to remove castling rights for the appropriate side if a rook moves from its original square.
            if matches!(future_piece.class,PieceClass::Rook){
                let (start_file,start_rank) = chess_move.vector.piece_at_start.location.get_file_rank();
                if start_file == 0{
                    if start_rank == 7 && matches!(future_piece.team,PieceTeam::Dark){
                        remove_castling_queenside_rights = true;
                    }else if start_rank == 0 && matches!(future_piece.team,PieceTeam::Light){
                        remove_castling_queenside_rights = true;
                    }   
                }else if start_file == 7{
                    if start_rank == 7 && matches!(future_piece.team,PieceTeam::Dark){
                        remove_castling_kingside_rights = true;
                    }else if start_rank == 0 && matches!(future_piece.team,PieceTeam::Light){
                        remove_castling_kingside_rights = true;
                    }  
                }
            }
        }
        MoveTypes::Castling(rook_vector) => {
            // Handle king movement
            let future_piece = result.piece_register.edit_piece_at_location(chess_move.vector.piece_at_start.location)?;
            future_piece.location = chess_move.vector.destination;

            // Handle rook movement
            let future_piece = result.piece_register.edit_piece_at_location(rook_vector.piece_at_start.location)?;
            future_piece.location = rook_vector.destination;

            // Flag to remove both castling rights after castling.
            remove_castling_kingside_rights = true;
            remove_castling_queenside_rights = true;
        }
        MoveTypes::DoubleStep(behind_pawn)=>{
            // Handle pawn movement
            let future_piece = result.piece_register.edit_piece_at_location(chess_move.vector.piece_at_start.location)?;
            future_piece.location = chess_move.vector.destination;

            // Mark en passant target square.
            result.special_flags.en_passant_location = Some(behind_pawn);
        }
        MoveTypes::EnPassant => {
            // Handle capture
            result.piece_register.remove_piece_at_location(chess_move.capture_status.expect("En passant should have placed this here").location)?;
        
            // Handle movement
            let future_piece = result.piece_register.edit_piece_at_location(chess_move.vector.piece_at_start.location)?;
            future_piece.location = chess_move.vector.destination;
        }
        MoveTypes::Promote(promoted_piece) => {
            // Move the piece, possibly capturing an enemy piece

            // Handle capture
            if let Some(x) = chess_move.capture_status{
                result.piece_register.remove_piece_at_location(x.location)?;
            }

            // Handle movement
            let future_piece = result.piece_register.edit_piece_at_location(chess_move.vector.piece_at_start.location)?;
            *future_piece = promoted_piece;
        } 
    }

    // Clear en passant flag unless a double-step was just performed.
    if !matches!(chess_move.move_type,MoveTypes::DoubleStep(_)){
        result.special_flags.en_passant_location = None;
    }

    // Update castling rights for the appropriate team and side.
    if remove_castling_kingside_rights{
        if matches!(chess_move.vector.piece_at_start.team,PieceTeam::Dark){
            result.special_flags.can_castle_king_dark = false;
        }else{
            result.special_flags.can_castle_king_light = false;
        }
    }
    if remove_castling_queenside_rights{
        if matches!(chess_move.vector.piece_at_start.team,PieceTeam::Dark){
            result.special_flags.can_castle_queen_dark = false;
        }else{
            result.special_flags.can_castle_queen_light = false;
        }
    }

    // Update half-move clock (for 50-move rule) and full-move count and turn
    if moving_a_pawn || capture_flag{
        result.move_counters.half_move_clock = 0;
    }else{
        result.move_counters.half_move_clock += 1;
    }
    if matches!(chess_move.vector.piece_at_start.team,PieceTeam::Dark){
        result.move_counters.full_move_count += 1;
        result.turn = PieceTeam::Light;
    }else{
        result.turn = PieceTeam::Dark;
    }
  
    Ok(result)
}

/// Applies an unchecked chess move to a given game state and filters if the move does not allow enemy check via resulting Option<GameState>.
/// This function handles all move types, including castling, en passant, promotion, and updates castling rights and clocks.
/// It will not update the game's check status
/// 
/// # Arguments
/// * `chess_move` - The move to apply.
/// * `game` - The current game state.
///
/// # Returns
/// * `Ok(Some(GameState))` - The new game state after the move is Some if it did not create friendly check
/// * `Err(Errors)` - If the move is invalid or cannot be applied.
pub fn apply_move_to_game_filtering_no_friendly_check(chess_move: &MoveDescription, game: &GameState) -> Result<Option<GameState>, ChessErrors> {
    // Do a regular game update
    let mut candidate_game = apply_move_to_game_unchecked(chess_move, game)?;
    // Now temporarily invert the turn to inspect for friendly check
    let turn_cache = candidate_game.turn; 
    candidate_game.turn = match turn_cache {
        PieceTeam::Dark => PieceTeam::Light,
        PieceTeam::Light => PieceTeam::Dark
    };
    if inspect_if_game_has_king_in_check(&candidate_game)?{
        Ok(None)
    }else{
        // No friendly check, set the turn back
        candidate_game.turn = turn_cache;
        Ok(Some(candidate_game))
    }
}