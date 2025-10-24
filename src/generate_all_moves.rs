use crate::{chess_errors::ChessErrors, game_state::GameState, generate_moves_level_5::{GenerateLevel5Result, generate_moves_level_5}, piece_team::PieceTeam};

/// Generates all chess moves
pub fn generate_all_moves(game: &GameState) -> Result<GenerateLevel5Result, ChessErrors> {
    let mut result = GenerateLevel5Result::new();
    if matches!(game.turn, PieceTeam::Light) {
        for (_, p) in &game.piece_register.light_pieces {
            result.extend(generate_moves_level_5(p, game)?);
        }
    } else {
        for (_, p) in &game.piece_register.dark_pieces {
            result.extend(generate_moves_level_5(p, game)?);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_generate_moves_level_5_extras() {
        let game =
            GameState::from_fen("8/2p5/3p4/KP5r/5R1k/8/4P1P1/8 b - - 1 1")
                .unwrap();
        let moves = generate_all_moves(&game).unwrap();
        assert_eq!(moves.len(), 2);

    }
}