use std::collections::LinkedList;

use crate::{
    apply_move_to_game::apply_move_to_game_unchecked,
    checked_move_description::CheckedMoveDescription, chess_errors::ChessErrors,
    collision_masks::CollisionMasks, game_state::GameState,
    generate_moves_level_3::GenerateLevel3Result, generate_moves_level_4::generate_moves_level_4,
    piece_record::PieceRecord, types_of_check::TypesOfCheck,
};

/// At level 5 we provided the rule checked move description and future game state after that move.
/// This layer can only find single checks, not double checks

pub struct CheckedMoveWithFutureGame {
    pub checked_move: CheckedMoveDescription,
    pub game_after_move: GameState,
}

type GenerateLevel5Result = LinkedList<CheckedMoveWithFutureGame>;

/// Level 5 filters the level 4 moves by inspecting for check and rules violations

pub fn generate_moves_level_5(
    piece: &PieceRecord,
    game: &GameState,
) -> Result<GenerateLevel5Result, ChessErrors> {
    let mut result = GenerateLevel5Result::new();
    let masks = CollisionMasks::from(&game.piece_register);
    let candidate_moves =
        generate_moves_level_4(piece, &masks, &game.special_flags, &game.piece_register)?;
    for move_to_try in candidate_moves {
        // Look for if this move created check
        // See if the piece's team's king is in check after this move
        let mut check_status: Option<TypesOfCheck> = None;

        // Get the enemy king
        let enemy_king_record = match piece.team {
            crate::piece_team::PieceTeam::Light => game.piece_register.dark_king,
            crate::piece_team::PieceTeam::Dark => game.piece_register.light_king,
        };

        // Simulate the future game
        let mut future_game = apply_move_to_game_unchecked(&move_to_try, game)?;

        // NOTE: This is a relativley slow and wasteful lookup, but cannot figure out how to cleanly cache this right now
        let future_piece = future_game
            .piece_register
            .view_piece_at_location(move_to_try.vector.destination)?;

        // Generate moves from the piece at the destination

        let future_moves_level_3 = GenerateLevel3Result::from(
            future_piece,
            &CollisionMasks::from(&future_game.piece_register),
        )?;

        // Look for a king capture in this level 3
        for f in future_moves_level_3.captures {
            // Capture collision with enemy king
            if f.binary_location & enemy_king_record.location.binary_location > 0 {
                check_status = Some(TypesOfCheck::SingleCheck(enemy_king_record, *future_piece));
                break;
            }
        }

        // Make sure the future game has the check status
        future_game.check_status = check_status;

        // Add the move description and future game
        result.push_back(CheckedMoveWithFutureGame {
            checked_move: CheckedMoveDescription {
                description: move_to_try,
                check_status,
            },
            game_after_move: future_game,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::board_location::BoardLocation;

    use super::*;

    #[test]
    fn test_king_moves_level_5() {
        let game =
            GameState::from_fen("rnbqkbnr/pppp1ppp/8/4p3/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2")
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
        let mut is_single_check = false;
        for m in moves {
            if let Some(x) = m.checked_move.check_status {
                check_count += 1;
                is_single_check = matches!(x, TypesOfCheck::SingleCheck(_, _));
            }
        }
        assert_eq!(check_count, 1);
        assert!(is_single_check); // This layer cannot inspect for double check or pins, so only reporting as single check
    }
}
