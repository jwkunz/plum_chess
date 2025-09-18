use crate::{chess_move::ChessMove, game_state::{GameState}};

pub trait ChessEngineThreadTrait{
    fn new() -> Self where Self: Sized;
    fn setup(&mut self, game : &GameState, calculation_time_s : f32);
    fn start_searching(&mut self);
    fn stop_searching(&mut self);
    fn is_done_searching(&self) -> bool;
    fn get_best_move(&self) -> Option<ChessMove>;
}