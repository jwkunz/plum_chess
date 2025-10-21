use std::{collections::VecDeque, sync::mpsc, time::Instant};

use rand::{seq::IteratorRandom};

use crate::{
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    }, chess_errors::ChessErrors, game_state::GameState, generate_moves_level_5::generate_all_moves, move_description::MoveDescription
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
    best_so_far: Option<MoveDescription>,
    /// Strings to print
    string_log: VecDeque<String>,
    /// IO
    command_receiver: mpsc::Receiver<EngineControlMessageType>,
    response_sender: mpsc::Sender<EngineResponseMessageType>,
}

impl ChessEngineThreadTrait for EngineRandom {
    fn configure(
        &mut self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ){
        self.starting_position = starting_position;
        self.calculation_time_s = calculation_time_s;
        self.command_receiver = command_receiver;
        self.response_sender = response_sender;
    }

    fn record_start_time(&mut self) {
        self.start_time = Instant::now();
    }

    fn compute_elapsed_micros(&self) -> u128 {
        (Instant::now() - self.start_time).as_micros()
    }

    fn set_status_calculating(&mut self, x: bool) {
        self.status_calculating = x;
    }

    fn get_status_calculating(&self) -> bool {
        self.status_calculating
    }

    fn get_command_receiver(&self) -> &mpsc::Receiver<EngineControlMessageType> {
        &self.command_receiver
    }

    fn get_response_sender(&self) -> &mpsc::Sender<EngineResponseMessageType> {
        &self.response_sender
    }

    fn get_best_move_so_far(&self) -> Option<MoveDescription> {
        self.best_so_far.clone()
    }

    fn add_string_to_print_log(&mut self, x: &str){
        self.string_log.push_back(x.to_string());
    }

    fn pop_next_string_to_log(&mut self) -> Option<String> {
        self.string_log.pop_front()
    }

    fn get_calculation_time_as_micros(&self) -> u128 {
        (self.calculation_time_s * 1E6).round() as u128
    }

    /// Pick a random move
    fn calculating_callback(&mut self) -> Result<(), ChessErrors> {
        if let Ok(moves) = generate_all_moves(&self.starting_position) {
            let mut rng = rand::rng();
            if let Some(random_move) = moves.iter().choose(&mut rng) {
                self.best_so_far = Some(random_move.checked_move.description.clone());
                self.set_status_calculating(false);
            }
        }
        Ok(())
    }
}

impl EngineRandom{
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineRandom {
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
            start_time: Instant::now(),
            status_calculating: false,
            best_so_far: None,
            string_log: VecDeque::new(),
        }
    }
}