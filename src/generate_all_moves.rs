use crate::{chess_errors::ChessErrors, game_state::GameState, generate_moves_level_5::{GenerateLevel5Result, generate_moves_level_5}, piece_team::PieceTeam};
use std::thread;

pub fn generate_all_moves(game: &GameState)-> Result<GenerateLevel5Result, ChessErrors>{
    //generate_all_moves_multithread(game)
    generate_all_moves_single_threaded(game) // Tested is generally faster when optimized compiled
}

/// Generates all chess moves
pub fn generate_all_moves_multithread(game: &GameState) -> Result<GenerateLevel5Result, ChessErrors> {
    let mut result = GenerateLevel5Result::new();

    // Collect references to the pieces we need to process
    let pieces: Vec<&_> = if matches!(game.turn, PieceTeam::Light) {
        game.piece_register.light_pieces.iter().map(|(_, p)| p).collect()
    } else {
        game.piece_register.dark_pieces.iter().map(|(_, p)| p).collect()
    };

    // Use scoped threads so we can safely borrow `game` and piece references.
    thread::scope(|s| -> Result<(), ChessErrors> {
        let mut handles = Vec::with_capacity(pieces.len());
        for p in pieces {
            // spawn a scoped thread that borrows `p` and `game`
            handles.push(s.spawn(move || generate_moves_level_5(p, game)));
        }

        for h in handles {
            // If a thread panicked this will panic here; otherwise propagate generate_moves_level_5 errors.
            let gen_moves = h.join().expect("generate_moves worker panicked")?;
            result.extend(gen_moves);
        }
        Ok(())
    })?;

    Ok(result)
}

/// Generates all chess moves
pub fn generate_all_moves_single_threaded(game: &GameState) -> Result<GenerateLevel5Result, ChessErrors> {
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