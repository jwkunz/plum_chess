use std::time::Instant;

use chrono::Duration;
use rand::{seq::IteratorRandom, thread_rng};

use crate::{
    chess_engine_thread_trait::ChessEngineThreadTrait, chess_move::ChessMove,
    game_state::GameState, move_logic::generate_all_moves,
};

pub struct EngineRandom {
    starting_position: Option<GameState>,
    calculation_time_s: Option<f32>,
    start_time: Instant,
    best_move: Option<ChessMove>,
    done_searching: bool,
}

impl ChessEngineThreadTrait for EngineRandom {
    fn new() -> Self {
        EngineRandom {
            starting_position: None,
            calculation_time_s: None,
            start_time: Instant::now(),
            best_move: None,
            done_searching: false,
        }
    }
    
    fn setup(&mut self, game: &crate::game_state::GameState, calculation_time_s: f32) {
        self.starting_position = Some(game.clone());
        self.calculation_time_s = Some(calculation_time_s);
        self.done_searching = false;
        self.best_move = None;
    }
    
    fn stop_searching(&mut self) {
        self.done_searching = true;
    }
    
    fn is_done_searching(&self) -> bool {
        self.done_searching
    }

    fn start_searching(&mut self) {
        self.start_time = Instant::now();
        if let Some(position) = &self.starting_position {
            if let Ok(moves) = generate_all_moves(&position) {
                let mut rng = thread_rng();
                if let Some(random_move) = moves.iter().choose(&mut rng) {
                    self.best_move = Some(random_move.description.clone());
                    self.done_searching = true;
                }
            }
        }
    }
    fn get_best_move(&self) -> Option<ChessMove> {
        self.best_move.clone()
    }
}
