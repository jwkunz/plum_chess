//! Chess engine thread interface and message passing utilities.
//!
//! This module defines the core interface for chess engine implementations that can:
//! - Run calculations in a separate thread
//! - Respond to control messages 
//! - Report progress and results
//! - Handle timeouts and graceful termination
//!
//! The design uses message passing between the engine thread and UCI interface:
//! - `EngineControlMessageType`: Commands sent to the engine
//! - `EngineResponseMessageType`: Responses and status updates from the engine
//!
//! The `ChessEngineThreadTrait` provides a minimal interface that concrete engines
//! must implement, allowing both direct usage and trait object boxing.

use std::{sync::mpsc, thread::sleep, time::Duration};
use crate::{scoring::CanScoreGame, chess_errors::ChessErrors, game_state::GameState, move_description::MoveDescription};

/// Control messages that can be sent to a chess engine thread.
///
/// These messages control the engine's execution and query its state:
/// - StartCalculating: Begin the move search
/// - AreYouStillCalculating: Poll if engine is still working
/// - GiveMeYourBestMoveSoFar: Request current best move
/// - GiveMeAStringToLog: Request next log message
/// - StopNow: Terminate the search
#[derive(Debug)]
pub enum EngineControlMessageType {
    StartCalculating,
    AreYouStillCalculating,
    GiveMeYourBestMoveSoFar,
    GiveMeAStringToLog,
    StopNow,
}

/// Response messages sent from the chess engine thread.
///
/// These messages provide status updates and results:
/// - BestMoveFound: Returns the current best move (if any)
/// - HadAnError: Reports an error condition
/// - StillCalculatingStatus: Reports if engine is still searching
/// - StringToLog: Provides next log message (if any)
#[derive(Debug)]
pub enum EngineResponseMessageType {
    BestMoveFound(Option<MoveDescription>),
    HadAnError(ChessErrors),
    StillCalculatingStatus(bool),
    StringToLog(Option<String>),
}

/// Trait defining the interface for chess engine implementations.
///
/// This trait provides the minimal interface needed by the UCI handler to:
/// - Configure an engine instance with a position and time control
/// - Start and stop calculations
/// - Monitor progress
/// - Retrieve results
///
/// The trait is designed to be object-safe (except for new()) to allow both
/// concrete and boxed dynamic usage. Engine implementations should handle:
/// - Position evaluation and move searching
/// - Time management
/// - Message handling and status reporting
/// - Graceful termination
///
/// # Type Parameters
/// The trait is marked as `Send` to allow thread safety.
///
/// # Examples
/// ```
/// use my_chess_engine::RandomEngine;
/// use plum_chess::{GameState, ChessEngineThreadTrait};
///
/// let mut engine = RandomEngine::new();
/// let game = GameState::new_game();
/// 
/// // Configure the engine
/// engine.configure(game, 5.0, command_rx, response_tx);
///
/// // Start calculation
/// engine.record_start_time();
/// engine.set_status_calculating(true);
///
/// // Poll until done
/// while engine.get_status_calculating() {
///     engine.tick();
/// }
///
/// // Get result
/// if let Some(best_move) = engine.get_best_move_so_far() {
///     println!("Best move found: {}", best_move);
/// }
/// ```
pub trait ChessEngineThreadTrait<T:CanScoreGame>: Send {
    /// Configures the engine with a position and search parameters.
    ///
    /// # Arguments
    /// * `starting_position` - The game position to analyze
    /// * `calculation_time_s` - Maximum search time in seconds
    /// * `command_receiver` - Channel for receiving control messages
    /// * `response_sender` - Channel for sending status updates
    /// * `scoring_object` - An object that can numerically score a game
    fn configure(
        &mut self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
        scoring_object: T
    );

    /// Records the search start time for managing time controls.
    fn record_start_time(&mut self);

    /// Computes elapsed microseconds since search started.
    ///
    /// # Returns
    /// Number of microseconds elapsed since record_start_time() was called
    fn compute_elapsed_micros(&self) -> u128;

    /// Sets the calculation status flag.
    ///
    /// # Arguments
    /// * `x` - True if engine is calculating, false otherwise
    fn set_status_calculating(&mut self, x: bool);

    /// Gets the current calculation status.
    ///
    /// # Returns
    /// True if engine is still calculating, false if idle or done
    fn get_status_calculating(&self) -> bool;

    /// Gets the command receiver channel.
    fn get_command_receiver(&self) -> &mpsc::Receiver<EngineControlMessageType>;

    /// Gets the response sender channel.
    fn get_response_sender(&self) -> &mpsc::Sender<EngineResponseMessageType>;

    /// Gets the current best move found during search.
    ///
    /// # Returns
    /// Some(MoveDescription) if a move was found, None otherwise
    fn get_best_move_so_far(&self) -> Option<MoveDescription>;

    /// Removes and returns the next pending log message.
    ///
    /// # Returns
    /// Some(String) if a message is available, None otherwise
    fn pop_next_string_to_log(&mut self) -> Option<String>;

    /// Adds a message to the log queue.
    ///
    /// # Arguments
    /// * `x` - The message to log
    fn add_string_to_print_log(&mut self, x: &str);

    /// Gets the maximum calculation time in microseconds.
    ///
    /// # Returns
    /// Maximum search time in microseconds
    fn get_calculation_time_as_micros(&self) -> u128;

    /// Performs one iteration of the engine's main loop.
    ///
    /// This method:
    /// 1. Checks for and handles incoming control messages
    /// 2. Checks for timeout conditions
    /// 3. Calls calculating_callback() if search is active
    /// 4. Sleeps briefly if idle
    ///
    /// The default implementation provides message handling and
    /// timing logic. Engine implementations only need to provide
    /// the calculating_callback() method.
    fn tick(&mut self) {
        let mut message_in = None;
        if let Ok(x) = self.get_command_receiver().try_recv(){
            message_in = Some(x);
        }

        match message_in{
            Some(EngineControlMessageType::StartCalculating)=>{
                self.set_status_calculating(true);
                self.record_start_time();
            }
            Some(EngineControlMessageType::AreYouStillCalculating) =>{
                let _ = self.get_response_sender().send(EngineResponseMessageType::StillCalculatingStatus(self.get_status_calculating()));
            }
            Some(EngineControlMessageType::StopNow) => {
                self.set_status_calculating(false);
            },
            Some(EngineControlMessageType::GiveMeYourBestMoveSoFar) => {
                let _ = self.get_response_sender().send(EngineResponseMessageType::BestMoveFound(self.get_best_move_so_far()));
            }
            Some(EngineControlMessageType::GiveMeAStringToLog) => {
                let log_string = self.pop_next_string_to_log();
                let _ = self.get_response_sender().send(EngineResponseMessageType::StringToLog(log_string));
            
            }
            _ => () // Ignore others
        }

        // Handle timeout
        if self.compute_elapsed_micros() >= self.get_calculation_time_as_micros(){
            self.set_status_calculating(false);
        }

        // Do calculation activities here
        if self.get_status_calculating() {
            if let Err(message) = self.calculating_callback(){
                let _ = self.get_response_sender().send(EngineResponseMessageType::HadAnError(message));
            }
        }else{
            sleep(Duration::from_millis(10));
        }
    }

    /// Performs one iteration of position analysis.
    ///
    /// This method is called repeatedly by tick() while the engine
    /// is calculating. Implementations should:
    /// - Analyze the current position
    /// - Update best move if better one found
    /// - Add relevant info strings to log
    /// - Return any errors encountered
    ///
    /// # Returns
    /// Ok(()) if iteration completed successfully
    /// Err(ChessErrors) if an error occurred
    fn calculating_callback(&mut self) -> Result<(), ChessErrors>;
}