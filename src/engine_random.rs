use std::{
    collections::VecDeque, sync::mpsc, thread::{self, JoinHandle, Thread}, time::{Duration, Instant}
};

use rand::{seq::IteratorRandom, thread_rng};

use crate::{
    chess_engine_thread_trait::{
        self, ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_move::ChessMove,
    errors::Errors,
    game_state::GameState,
    move_logic::generate_all_moves,
};

/// A trivial, purely random engine implementation used for testing and as a reference engine.
///
/// EngineRandom implements the `ChessEngineThreadTrait` and selects a legal move at random
/// from the current position when asked to search. It is intentionally simple:
/// - It does not perform any real search or evaluation.
/// - It clones and stores the starting position on setup and chooses a move immediately
///   when `start_searching` is invoked (synchronously).
/// - It supports the minimal lifecycle required by the trait: `setup`, `start_searching`,
///   `stop_searching`, `is_done_searching`, and `get_best_move`.
///
/// This engine is useful for:
/// - Unit tests that need a deterministic or cheap engine alternative (random but safe).
/// - Exercising the UCI handler and threading logic without implementing a complex engine.
/// - Providing a concrete implementation for APIs that expect an engine instance.
pub struct EngineRandom {
    /// The cloned game state provided during `setup`. None until setup is called.
    starting_position: GameState,
    /// Requested calculation time in seconds. None until setup is called.
    calculation_time_s: f32,
    /// The instant at which a search was started. Used to emulate timing behavior.
    start_time: Instant,
    /// Calculation status
    status_calculating: bool,
    /// Best move so far
    best_so_far: Option<ChessMove>,
    /// Strings to print
    string_log : VecDeque<String>,
    /// IO
    command_receiver: mpsc::Receiver<EngineControlMessageType>,
    response_sender: mpsc::Sender<EngineResponseMessageType>,
}

impl ChessEngineThreadTrait for EngineRandom {
    fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineRandom::new(starting_position, calculation_time_s, command_receiver, response_sender)
    }

    fn record_start_time(&mut self){
        self.start_time = Instant::now();
    }

    fn compute_elapsed_micros(&self) -> u128{
        (Instant::now() - self.start_time).as_micros()
    }

    fn set_status_calculating(&mut self, x : bool){
        self.status_calculating = x;
    }

    fn get_status_calculating(&self) -> bool{
        self.status_calculating
    }

    fn get_command_receiver(&self) -> &mpsc::Receiver<EngineControlMessageType>{
        &self.command_receiver
    }

    fn get_response_sender(&self) -> &mpsc::Sender<EngineResponseMessageType>{
        &self.response_sender
    }

    fn get_best_move_so_far(&self) -> Option<ChessMove>{
        self.best_so_far.clone()
    }

    fn add_string_to_print_log(&mut self, x : String) -> Result<(),Errors>{
        self.string_log.push_back(x);
        Ok(())
    }

    fn pop_next_string_to_log(&mut self) -> Option<String>{
        self.string_log.front().cloned()
    }

    fn get_calculation_time_as_micros(&self) -> u128{
        (self.calculation_time_s * 1E6).round() as u128
    }

    fn calculating_callback(&mut self) -> Result<(), Errors> {
        if let Ok(moves) = generate_all_moves(&self.starting_position) {
            let mut rng = thread_rng();
            if let Some(random_move) = moves.iter().choose(&mut rng) {
                self.best_so_far = Some(random_move.description.clone());
                self.set_status_calculating(false);
            }
        }
        Ok(())
    }
}


impl EngineRandom {
    fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineRandom {
            starting_position,
            calculation_time_s,
            start_time: Instant::now(),
            status_calculating: false,
            best_so_far: None,
            string_log: VecDeque::new(),
            command_receiver,
            response_sender,
        }
    }
}