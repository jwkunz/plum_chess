use std::collections::LinkedList;
#[allow(unused_imports)]
use crate::{
    apply_move_to_game::apply_move_to_game_filtering_no_friendly_check, 
    checked_move_description::CheckedMoveDescription, 
    chess_errors::ChessErrors, 
    collision_masks::CollisionMasks, 
    game_state::GameState, 
    generate_moves_level_4::generate_moves_level_4, 
    inspect_check::{inspect_check}, 
    piece_record::PieceRecord, 
    piece_team::PieceTeam, 
    
    types_of_check::TypesOfCheck::SingleCheck
};


/// At level 5 we provided the rule checked move description and future game state after that move.
/// This layer can only find single checks, not double checks
/// This layer does inspect to make sure a move does leave the friendly king in check
#[derive(Clone, Debug)]
pub struct CheckedMoveWithFutureGame {
    pub checked_move: CheckedMoveDescription,
    pub game_after_move: GameState,
}

pub type GenerateLevel5Result = LinkedList<CheckedMoveWithFutureGame>;

/// Level 5 filters the level 4 moves by inspecting for check and rules violations

pub fn generate_moves_level_5(
    piece: &PieceRecord,
    game: &GameState,
) -> Result<GenerateLevel5Result, ChessErrors> {
    let mut result = GenerateLevel5Result::new();
    let masks = CollisionMasks::from(&game.piece_register);
    let candidate_moves =
        generate_moves_level_4(piece, &masks, &game.special_flags, &game.piece_register)?;
    // For each move
    for move_to_try in candidate_moves {
        //dbg!(format!("Move: {:?} in {:?}", move_to_try.get_long_algebraic(), game.get_fen()));
        // Simulate the future game, and make sure it doesn't create friendly check
        if let Some(future_game) =
            apply_move_to_game_filtering_no_friendly_check(&move_to_try, game)?
        {
            // Inspection for enemy check
            let last_piece_moved_option = *future_game.piece_register.view_piece_at_location(move_to_try.vector.destination)?;
            let check_status = inspect_check(&future_game, Some(last_piece_moved_option))?;
   
            // Add the move description and future game
            result.push_back(CheckedMoveWithFutureGame {
                checked_move: CheckedMoveDescription {
                    description: move_to_try,
                    check_status,
                },
                game_after_move: future_game,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::board_location::BoardLocation;

    use super::*;

    #[test]
    fn test_generate_moves_level_5() {
        // A game that allows check and has a pinned piece
        let game =
            GameState::from_fen("rnb1kbnr/pppp1ppp/8/4p3/3PP2q/8/PPP2PPP/RNBQKBNR w KQkq - 1 3")
                .unwrap();
        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("f8").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 5);
        let mut check_count = 0;
        for m in moves {
            if m.checked_move.check_status.is_some() {
                check_count += 1;
            }
        }
        assert_eq!(check_count, 1);

        // Should not allow pinned piece to move
        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("f2").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 0);

        // A double check scenario
        let game = GameState::from_fen("8/6k1/8/4pp2/3q4/6B1/p7/5KR1 w - - 0 1").unwrap();
        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("g3").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 6);
        let mut check_count = 0;
        for m in moves {
            if let Some(_) = m.checked_move.check_status {
                check_count += 1;
            }
        }
        assert_eq!(check_count, 6);

        // A one move check scenario
        let game =
            GameState::from_fen("rnb1kbnr/pppp1ppp/8/4p3/3Pq3/2P5/PP3PPP/RNBQKBNR w KQkq - 0 4")
                .unwrap();
        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("e1").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 1);

        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("f1").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 1);

        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("g1").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 1);

        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("d1").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 1);

        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("c1").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 1);

        let moves = generate_moves_level_5(
            game.piece_register
                .view_piece_at_location(BoardLocation::from_long_algebraic("f2").unwrap())
                .unwrap(),
            &game,
        )
        .unwrap();
        assert_eq!(moves.len(), 0);
    }
}
