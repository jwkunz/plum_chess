use std::{collections::VecDeque, sync::mpsc, time::Instant};

use rand::{seq::IteratorRandom};

use crate::{
    scoring::CanScoreGame, chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    }, chess_errors::ChessErrors, game_state::GameState, generate_all_moves::generate_all_moves, move_description::MoveDescription
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
pub struct EngineRandom<T:CanScoreGame> {
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
    // Scoring object
    scoring_object : T
}

/// Implementation of the ChessEngineThreadTrait for EngineRandom.
///
/// This impl provides all lifecycle, timing, logging, and messaging helpers
/// required for an engine thread that selects moves at random. It is intended
/// for use in situations where a lightweight, non-deterministic move chooser
/// is sufficient (for testing, fuzzing, or as a baseline).
///
/// Behavior summary:
/// - configure: initialize the thread state (starting position, time limit,
///   and channel endpoints for control/response).
/// - record_start_time / compute_elapsed_micros: capture and query elapsed time
///   using std::time::Instant (microsecond precision).
/// - set_status_calculating / get_status_calculating: track whether the engine
///   is currently performing a calculation.
/// - get_command_receiver / get_response_sender: provide access to the control
///   and response channels used to communicate with the engine thread.
/// - get_best_move_so_far: return the last chosen move (cloned Option<MoveDescription>).
/// - add_string_to_print_log / pop_next_string_to_log: enqueue and dequeue
///   textual log entries in a FIFO manner.
/// - get_calculation_time_as_micros: convert the configured calculation time
///   (seconds as f32) into microseconds as u128, rounding to the nearest microsecond.
/// - calculating_callback: the core routine for this engine; it generates all
///   legal moves for the configured starting position and picks one uniformly
///   at random. If a move is found it records it as the best-so-far and clears
///   the calculating flag; if no legal moves are available it returns
///   ChessErrors::NoLegalMoves. Any errors from move generation are propagated.
///
/// Notes and considerations:
/// - This implementation assumes single-threaded ownership of EngineRandom's
///   fields; concurrency-safe wrappers are not provided here.
/// - Randomness source is obtained via rand::rng(); if deterministic behavior
///   is required (reproducible tests), ensure the RNG is seeded appropriately
///   or replaced with a deterministic generator.
/// - Time arithmetic uses Instant, which is monotonic but not related to wall-clock time.
/// - Error propagation: callers should handle ChessErrors coming from move generation.
///
/// Typical usage:
/// 1. configure the engine thread with a position, calculation time, and channels.
/// 2. call record_start_time() before beginning calculation.
/// 3. invoke calculating_callback() to pick a move; use get_best_move_so_far()
///    to retrieve the result and use get_response_sender() / get_command_receiver()
///    to integrate with the engine's control loop.
impl <T:CanScoreGame> ChessEngineThreadTrait<T> for EngineRandom<T> {
    fn configure(
        &mut self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
        scoring_object: T
    ){
        self.starting_position = starting_position;
        self.calculation_time_s = calculation_time_s;
        self.command_receiver = command_receiver;
        self.response_sender = response_sender;
        self.scoring_object = scoring_object;
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
        let moves = generate_all_moves(&self.starting_position)?;        
        let mut rng = rand::rng();
        if let Some(random_move) = moves.iter().choose(&mut rng) {
            self.best_so_far = Some(random_move.checked_move.description.clone());
            self.set_status_calculating(false);
        }else{
            return Err(ChessErrors::NoLegalMoves);
        }
        Ok(())
    }
}

impl <T:CanScoreGame> EngineRandom<T>{
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
        scoring_object : T
    ) -> Self {
        EngineRandom::<T> {
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
            start_time: Instant::now(),
            status_calculating: false,
            best_so_far: None,
            string_log: VecDeque::new(),
            scoring_object
        }
    }
}