//! Utilities for applying a MoveDescription to a GameState.
//!
//! This module contains two public helpers used by the move generation and
//! validation pipeline:
//!
//! - apply_move_to_game_unchecked: performs an in-place logical application of
//!   a move against a cloned GameState and returns the resulting GameState.
//!   This function performs the piece movements, captures, promotions,
//!   en-passant bookkeeping, castling rook movement, castling-rights updates,
//!   and move-clock updates. It does NOT validate whether the move leaves the
//!   moving side in check (i.e. it does not perform legality tests) — callers
//!   must handle check detection if they require fully-legal move semantics.
//!
//! - apply_move_to_game_filtering_no_friendly_check: a thin wrapper that
//!   applies a move and then rejects the result if the moving side would be
//!   left in check (including special handling for castling to ensure no king
//!   passing square is attacked).
//!
//! Both functions return ChessErrors on invalid internal operations (for
//! example if piece_register operations fail). The filtering variant returns
//! Ok(None) when the move is legal in terms of board operations but illegal
//! because it results in the moving side's king being in check (or because
//! a castling move would pass through or start in check).
//!
//! Implementation notes / invariants:
//! - These functions operate on a cloned GameState and never mutate the
//!   provided `game` argument.
//! - apply_move_to_game_unchecked assumes MoveDescription is syntactically
//!   valid for the given GameState; it will still return errors if the
//!   underlying piece-register operations fail.
//! - En passant handling requires MoveDescription::capture_status to be set
//!   for MoveTypes::EnPassant — the current implementation uses an `expect`
//!   and will panic if that invariant is violated. Callers constructing
//!   MoveDescription for en-passant should ensure the capture_status is
//!   present.
//! - The functions update special flags (en-passant target, castling rights)
//!   and move counters (halfmove clock, fullmove count) according to FIDE
//!   style rules used in this engine.
//!
//! See the individual function docs for more-detailed behavior for each move
//! type (regular, double pawn step, en-passant, castling, promotion).
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

/// Apply a MoveDescription to a GameState without performing a legality check.
///
/// This function performs the low-level application of a chess move, returning
/// a brand-new GameState with all of the engine bookkeeping updated. It is
/// intentionally "unchecked": it does not guarantee that the resulting position
/// leaves the moving side's king out of check. Use
/// `apply_move_to_game_filtering_no_friendly_check` when you need to reject
/// moves that leave the mover in check.
///
/// The function handles the following move types:
/// - MoveTypes::Regular:
///   - Moves a piece from the start to destination, overwriting (capturing)
///     any piece at the destination using the piece_register helpers.
///   - If the moved piece is a King or a Rook from its original corner rank/file,
///     the corresponding castling rights on the moving side are cleared.
/// - MoveTypes::Castling(rook_vector):
///   - Moves the king to its castled square and moves the rook according to the
///     provided rook_vector. Clears both king- and queen-side castling rights
///     for the moving side.
/// - MoveTypes::DoubleStep(behind_pawn):
///   - Moves a pawn two squares forward and sets the en-passant target square
///     (`special_flags.en_passant_location`) to `behind_pawn` (the square behind
///     the pawn where an en-passant capture may occur next ply).
/// - MoveTypes::EnPassant:
///   - Expects `chess_move.capture_status` to contain the captured pawn's
///     location (the square behind the double-stepped pawn). Removes the
///     captured pawn and moves the en-passant capturing pawn to the destination.
///   - NOTE: an internal `expect` is used when reading capture_status; if the
///     MoveDescription is malformed (missing capture_status) this will panic.
///     Construct MoveDescription for en-passant carefully.
/// - MoveTypes::Promote(promoted_piece):
///   - Moves the pawn to the promotion square and edits the piece at that
///     location to the promoted piece class. Capture is handled via the
///     overwrite semantics of the piece register move.
///
/// Additionally, the function updates:
/// - special_flags.en_passant_location: cleared for all move types except
///   DoubleStep (where it is explicitly set).
/// - special_flags.can_castle_* flags: cleared when the king or an original
///   rook moves (or any castling move occurs).
/// - move_counters.half_move_clock: reset to 0 when a pawn moves or a capture
///   occurs; otherwise incremented by 1.
/// - move_counters.full_move_count: incremented after Dark (black) completes a
///   move (i.e., when the mover was Dark).
/// - turn: flipped to the opposing team after the move is applied.
///
/// Errors:
/// - Returns ChessErrors propagated from the piece_register operations, or any
///   validation performed by internal helper methods called here.
///
/// Panics:
/// - If MoveTypes::EnPassant is used but the MoveDescription lacks
///   capture_status, this function will panic due to an `expect` call.
///
/// Example:
/// let new_state = apply_move_to_game_unchecked(&move_desc, &old_state)?;
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

/// Apply a MoveDescription and reject results that leave the mover in check.
///
/// This wrapper performs two main responsibilities in addition to what
/// `apply_move_to_game_unchecked` does:
///
/// 1. Ensures castling does not occur when the king is currently in check or
///    would pass through an attacked square. FIDE castling legality rules
///    require that:
///      - the king is not in check in the starting square,
///      - the squares the king traverses (and the destination) are not attacked.
///
///    Because castling is represented by a single composite MoveDescription,
///    this function breaks castling into intermediate regular king moves for
///    the passing squares and checks each one for check using `inspect_check`.
///
/// 2. After applying the given move, it checks whether the mover's king is in
///    check in the resulting position. If so, the function returns Ok(None),
///    signalling the move is illegal for the moving side. Otherwise it returns
///    Ok(Some(GameState)) with the legal updated position.
///
/// Behavior details:
/// - For castling, the function computes the list of king-passing squares
///   (for example: e1 -> f1 -> g1 for light kingside) and simulates moving
///   the king to each passing square in isolation, checking for check. If any
///   passing square is attacked (or inspect_check returns Some), the castling
///   move is rejected by returning Ok(None).
/// - After applying any move, the function temporarily flips candidate_game.turn
///   and calls inspect_check to ask whether the moving side's king is in check
///   in the resulting position. If check is present, the move is not legal and
///   Ok(None) is returned. If check is absent, Ok(Some(GameState)) is returned.
/// - The function preserves ChessErrors from underlying operations and returns
///   them as Err(ChessErrors).
///
/// Returns:
/// - Ok(Some(GameState)) when the move was applied successfully and does not
///   leave the mover in check.
/// - Ok(None) when the move would leave the mover in check or castling passes
///   through an attacked square (i.e., the move is illegal).
/// - Err(ChessErrors) for operational failures (invalid piece operations, bad
///   board coordinates, or other internal errors).
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

        // Another capture
        let new_game = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        let move_text = "b4f4";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"8/2p5/3p4/KP5r/5R1k/8/4P1P1/8 b - - 0 1");
        assert_eq!(updated_game.piece_register.dark_pieces.len(),4);
        assert_eq!(updated_game.piece_register.light_pieces.len(),5);

        // Another capture
        let new_game = GameState::from_fen("1rbnkbnr/pppp1ppp/8/8/4P2q/2N3P1/PPP2P1P/R1BQKBNR w KQk - 3 7").unwrap();
        let move_text = "g3h4";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap().unwrap();
        assert_eq!(updated_game.get_fen(),"1rbnkbnr/pppp1ppp/8/8/4P2P/2N5/PPP2P1P/R1BQKBNR b KQk - 0 7");

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

        // Pin
        let new_game = GameState::from_fen("8/2p5/3p4/KP5r/1R3p2/6Pk/4P3/8 w - - 1 2").unwrap();
        let move_text = "b5b6";
        let move_description = MoveDescription::from_long_algebraic(move_text, &new_game).unwrap();
        let updated_game =
            apply_move_to_game_filtering_no_friendly_check(&move_description, &new_game).unwrap();
        assert!(updated_game.is_none());
    }


    #[test]
    fn test_setup_complex_position(){
        let mut game = GameState::from_fen("rn1qkbnr/ppp1pppp/8/3p1b2/3P4/4P3/PPP2PPP/RNBQKBNR w KQkq - 1 3").unwrap();
        let moves = vec![" c2c3","f5b1","a1b1","e7e6","g1f3","f7f6","f1b5","b8d7","e1g1","f8a3","b2a3","e8f7","d1d3","b7b6","e3e4","d5e4","d3e4","d7e5","d4e5","f6e5","f3e5","f7f6","c1f4","g7g6","b1d1","d8d1","f1d1","h7h6","e5d7"];
        for move_description in moves {
            game = apply_move_to_game_unchecked(&MoveDescription::from_long_algebraic(move_description, &game).unwrap(),&game).unwrap();
        }
        let generated_moves = crate::generate_all_moves::generate_all_moves(&game).unwrap();
        assert_eq!(generated_moves.len(),3);
    }
}  