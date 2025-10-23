use crate::{
    board_location::BoardLocation,
    chess_errors::ChessErrors,
    game_state::GameState,
    inspect_check::{inspect_check},
    move_description::{
        MoveDescription,
        MoveTypes::{self, Castling},
        MoveVector,
    },
    piece_class::PieceClass,
    piece_team::PieceTeam,
};

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
pub fn apply_move_to_game_unchecked(
    chess_move: &MoveDescription,
    game: &GameState,
) -> Result<GameState, ChessErrors> {
    let mut result = game.clone();
    let mut remove_castling_kingside_rights = false;
    let mut remove_castling_queenside_rights = false;
    let mut capture_flag = false;
    let moving_a_pawn = matches!(chess_move.vector.piece_at_start.class, PieceClass::Pawn);

    // Handle the move based on its specialness (regular, castling, promotion, etc.)
    match chess_move.move_type {
        MoveTypes::Regular => {
            // Move the piece, possibly capturing an enemy piece
            let captured_piece = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    chess_move.vector.piece_at_start.location,
                    chess_move.vector.destination,
                )?;
            capture_flag = captured_piece.is_some();
            let future_piece = result
                .piece_register
                .view_piece_at_location(chess_move.vector.destination)?;

            // Remove castling rights if a king or rook moves.
            if matches!(future_piece.class, PieceClass::King) {
                remove_castling_kingside_rights = true;
                remove_castling_queenside_rights = true;
            }
            // Flag to remove castling rights for the appropriate side if a rook moves from its original square.
            if matches!(future_piece.class, PieceClass::Rook) {
                let (start_file, start_rank) =
                    chess_move.vector.piece_at_start.location.get_file_rank();
                if start_file == 0 {
                    if start_rank == 7 && matches!(future_piece.team, PieceTeam::Dark) {
                        remove_castling_queenside_rights = true;
                    } else if start_rank == 0 && matches!(future_piece.team, PieceTeam::Light) {
                        remove_castling_queenside_rights = true;
                    }
                } else if start_file == 7 {
                    if start_rank == 7 && matches!(future_piece.team, PieceTeam::Dark) {
                        remove_castling_kingside_rights = true;
                    } else if start_rank == 0 && matches!(future_piece.team, PieceTeam::Light) {
                        remove_castling_kingside_rights = true;
                    }
                }
            }
        }
        MoveTypes::Castling(rook_vector) => {
            // Handle king movement
            let _ = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    chess_move.vector.piece_at_start.location,
                    chess_move.vector.destination,
                )?;

            // Handle rook movement
            let _ = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    rook_vector.piece_at_start.location,
                    rook_vector.destination,
                )?;

            // Flag to remove both castling rights after castling.
            remove_castling_kingside_rights = true;
            remove_castling_queenside_rights = true;
        }
        MoveTypes::DoubleStep(behind_pawn) => {
            // Handle pawn movement
            let _ = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    chess_move.vector.piece_at_start.location,
                    chess_move.vector.destination,
                )?;

            // Mark en passant target square.
            result.special_flags.en_passant_location = Some(behind_pawn);
        }
        MoveTypes::EnPassant => {
            // Handle capture
            result.piece_register.remove_piece_at_location(
                chess_move
                    .capture_status
                    .expect("En passant should have placed this here")
                    .location,
            )?;

            // Handle movement
            let _ = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    chess_move.vector.piece_at_start.location,
                    chess_move.vector.destination,
                )?;
        }
        MoveTypes::Promote(promoted_piece) => {
            // Move the piece, possibly capturing an enemy piece

            // Handle movement
            let captured_piece = result
                .piece_register
                .move_piece_to_location_with_overwrite(
                    chess_move.vector.piece_at_start.location,
                    chess_move.vector.destination,
                )?;
            capture_flag = captured_piece.is_some();
            let future_piece = result
                .piece_register
                .edit_piece_at_location(chess_move.vector.destination)?;
            future_piece.class = promoted_piece.class;
        }
    }

    // Clear en passant flag unless a double-step was just performed.
    if !matches!(chess_move.move_type, MoveTypes::DoubleStep(_)) {
        result.special_flags.en_passant_location = None;
    }

    // Update castling rights for the appropriate team and side.
    if remove_castling_kingside_rights {
        if matches!(chess_move.vector.piece_at_start.team, PieceTeam::Dark) {
            result.special_flags.can_castle_king_dark = false;
        } else {
            result.special_flags.can_castle_king_light = false;
        }
    }
    if remove_castling_queenside_rights {
        if matches!(chess_move.vector.piece_at_start.team, PieceTeam::Dark) {
            result.special_flags.can_castle_queen_dark = false;
        } else {
            result.special_flags.can_castle_queen_light = false;
        }
    }

    // Update half-move clock (for 50-move rule) and full-move count and turn
    if moving_a_pawn || capture_flag {
        result.move_counters.half_move_clock = 0;
    } else {
        result.move_counters.half_move_clock += 1;
    }
    if matches!(chess_move.vector.piece_at_start.team, PieceTeam::Dark) {
        result.move_counters.full_move_count += 1;
        result.turn = PieceTeam::Light;
    } else {
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
pub fn apply_move_to_game_filtering_no_friendly_check(
    chess_move: &MoveDescription,
    game: &GameState,
) -> Result<Option<GameState>, ChessErrors> {

    // Special check handling for castling passing squares
    if matches!(chess_move.move_type, Castling(_)) {
        // Make sure current king is not in check
        if inspect_check(&game, None)?.is_some() {
            return Ok(None);
        }

        let square_list: Vec<&str>;
        if chess_move.vector.destination.binary_location
            == BoardLocation::from_long_algebraic("c1")?.binary_location
        {
            // Queenside castling for light
            square_list = vec!["c1", "d1"];
        } else if chess_move.vector.destination.binary_location
            == BoardLocation::from_long_algebraic("g1")?.binary_location
        {
            // Kingside castling for light
            square_list = vec!["f1", "g1"];
        } else if chess_move.vector.destination.binary_location
            == BoardLocation::from_long_algebraic("c8")?.binary_location
        {
            // Queenside castling for dark
            square_list = vec!["c8", "d8"];
        } else {
            // Kingside castling for dark
            square_list = vec!["f8", "g8"];
        }
        for squares in square_list {
            let passing_square = BoardLocation::from_long_algebraic(squares)?;
            let move_description = MoveDescription {
                vector: MoveVector {
                    piece_at_start: chess_move.vector.piece_at_start,
                    destination: passing_square,
                },
                move_type: MoveTypes::Regular,
                capture_status: None,
            };
            let mut temp_game = apply_move_to_game_unchecked(&move_description, &game)?;
            temp_game.turn = game.turn;
            if inspect_check(&temp_game, None)?.is_some() {
                return Ok(None);
            }
        }
    }

    
    // Do a regular game update
    let mut candidate_game = apply_move_to_game_unchecked(chess_move, game)?;
    // Now temporarily invert the turn to inspect for friendly check
    let turn_cache = candidate_game.turn;
    candidate_game.turn = match turn_cache {
        PieceTeam::Dark => PieceTeam::Light,
        PieceTeam::Light => PieceTeam::Dark,
    };
    if inspect_check(&candidate_game, None)?.is_some() {
        Ok(None)
    } else {
        // No friendly check, set the turn back
        candidate_game.turn = turn_cache;
        Ok(Some(candidate_game))
    }
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_apply_move_to_game_checked() {
        // Simple move
        let new_game = GameState::new_game();
        let move_text = "e2e4";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        
        // Simple capture
        let new_game = GameState::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2").unwrap();
        let move_text = "e4d5";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnbqkbnr/ppp1pppp/8/3P4/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2");
        
        // Blocked King
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/2P5/PP2NnPP/RNBQK2R b KQ - 0 8").unwrap();
        let move_text = "f8e8";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap();
        assert!(updated_game.is_none());

        // Simple Castling
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        let move_text = "e1g1";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQ1RK1 b - - 2 8");

        // Blocked Castling 1
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/4B2n/PPP1N1PP/RN1QK2R w KQ - 3 9").unwrap();
        let move_text = "e1g1";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap();
        assert!(updated_game.is_none());

        // Blocked Castling 2
        let new_game = GameState::from_fen("rnbq1k1r/pp1P3p/2p2p2/6p1/2BQ1b2/2N5/PPP1NnPP/R3K2R w KQ - 0 12").unwrap();
        let move_text = "e1c1";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap();
        assert!(updated_game.is_none());

        // Alowed Castling 2
        let new_game = GameState::from_fen("rnbq1k1r/pp1P3p/2p2p2/6p1/2B2Qn1/2N5/PPP1N1PP/R3K2R w KQ - 1 13").unwrap();
        let move_text = "e1c1";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnbq1k1r/pp1P3p/2p2p2/6p1/2B2Qn1/2N5/PPP1N1PP/2KR3R b - - 2 13");

        // No castling from check
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/3nB3/PPP1N1PP/RN1QK2R w KQ - 3 9").unwrap();
        let move_text = "e1g1";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap();
        assert!(updated_game.is_none());
    
        // Capture and promote
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        let move_text = "d7c8q";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnQq1k1r/pp2bppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R b KQ - 0 8");

        // Complex capture checkmate
        let new_game = GameState::from_fen("rnb1qk1r/pp1Pbppp/8/1Bp5/8/2P5/PP2NnPP/RNBQK2R w KQ - 0 10").unwrap();
        let move_text = "d7e8r";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnb1Rk1r/pp2bppp/8/1Bp5/8/2P5/PP2NnPP/RNBQK2R b KQ - 0 10");
        // Attempt to move after checkmate
        let move_text = "f7f5";
        let move_description = MoveDescription::from_long_algebraic(move_text, &updated_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &updated_game).unwrap();
        assert!(updated_game.is_none());

        // Simple en passant
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pb1pp/2p5/8/2B2pP1/2P5/PP2Nn1P/RNBQ1RK1 b - g3 0 10").unwrap();
        let move_text = "f4g3";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"rnbq1k1r/pp1Pb1pp/2p5/8/2B5/2P3p1/PP2Nn1P/RNBQ1RK1 w - - 0 11");

        // No en passant
        let new_game = GameState::from_fen("rnbq1k1r/pp1Pb2p/2p3p1/8/2B2pP1/2P4P/PP2Nn2/RNBQ1RK1 b - - 0 11").unwrap();
        let move_text = "f4g3";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &updated_game);
        assert!(updated_game.is_err());
    }
}   