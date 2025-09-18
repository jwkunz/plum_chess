use std::time::Instant;

use chrono::Duration;
use rand::{seq::IteratorRandom, thread_rng};

use crate::{
    chess_engine_thread_trait::ChessEngineThreadTrait, chess_move::ChessMove,
    game_state::GameState, move_logic::generate_all_moves,
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
    starting_position: Option<GameState>,
    /// Requested calculation time in seconds. None until setup is called.
    calculation_time_s: Option<f32>,
    /// The instant at which a search was started. Used to emulate timing behavior.
    start_time: Instant,
    /// The best move selected by the engine (if any).
    best_move: Option<ChessMove>,
    /// Flag indicating whether the engine has finished its current search.
    done_searching: bool,
}

impl ChessEngineThreadTrait for EngineRandom {
    /// Construct a new, uninitialized EngineRandom instance.
    ///
    /// Note: this constructor initializes fields to sensible defaults but does not start any
    /// background threads or begin any searching. Callers should call `setup` followed by
    /// `start_searching` to perform a calculation.
    fn new() -> Self {
        EngineRandom {
            starting_position: None,
            calculation_time_s: None,
            start_time: Instant::now(),
            best_move: None,
            done_searching: false,
        }
    }
    
    /// Prepare the engine for a calculation.
    ///
    /// The engine clones and stores the provided `game` state and records the requested
    /// calculation time in seconds. The engine resets any previous search result and
    /// clears the `done_searching` flag so a subsequent `start_searching` will perform work.
    ///
    /// This method is synchronous and lightweight.
    fn setup(&mut self, game: &crate::game_state::GameState, calculation_time_s: f32) {
        self.starting_position = Some(game.clone());
        self.calculation_time_s = Some(calculation_time_s);
        self.done_searching = false;
        self.best_move = None;
    }
    
    /// Signal the engine to stop searching.
    ///
    /// For this simple engine this sets the `done_searching` flag. If a real search loop
    /// existed it would use this hint to interrupt work and return quickly.
    fn stop_searching(&mut self) {
        self.done_searching = true;
    }
    
    /// Return whether the engine has finished its current search.
    ///
    /// For EngineRandom this is a simple boolean flag toggled by `start_searching` or
    /// `stop_searching`.
    fn is_done_searching(&self) -> bool {
        self.done_searching
    }

    /// Start the search and determine a best move.
    ///
    /// This implementation runs synchronously: it records the start time and, if a position
    /// was provided via `setup`, generates all legal moves for that position and picks one
    /// uniformly at random. The selected move is stored in `best_move` and the `done_searching`
    /// flag is set to true when the selection completes.
    ///
    /// Notes:
    /// - This method does not spawn a background thread; callers expecting asynchronous
    ///   behavior must run the engine in a separate thread if needed.
    /// - The engine relies on `move_logic::generate_all_moves` to enumerate legal moves.
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

    /// Return the best move found by the last search, if any.
    ///
    /// The returned `ChessMove` is cloned from the internal storage. `None` indicates that
    /// no move was selected (for example, if no legal moves exist or `setup` was not called).
    fn get_best_move(&self) -> Option<ChessMove> {
        self.best_move.clone()
    }
}
