//! A minimax-based chess engine implementation.
//!
//! This module provides a chess engine that uses the minimax algorithm to evaluate
//! positions and select moves. The implementation includes:
//! - An EngineMinimax struct implementing ChessEngineThreadTrait for integration
//!   with the engine control system
//! - A recursive minimax implementation that evaluates positions based on material
//!   count and propagates scores up the game tree
//! - Support for configurable search depth and time control
//!
//! The engine uses material counting as its primary evaluation metric and implements
//! alpha-beta pruning for improved search efficiency.

use std::{collections::VecDeque, sync::mpsc, time::Instant};

use crate::{
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_errors::ChessErrors,
    game_state::GameState,
    generate_all_moves::generate_all_moves,
    generate_moves_level_5::CheckedMoveWithFutureGame,
    move_description::MoveDescription,
    piece_team::PieceTeam,
    scoring::{
        MAX_SCORE, MIN_SCORE, Score, ScoringTypes, generate_losing_score, score_game
    },
};

/// A chess engine implementation using the minimax algorithm for move selection.
///
/// This engine evaluates positions by looking ahead a configurable number of moves
/// and selecting the move that leads to the best material count for the current player.
/// There are two configurable parameters for EngineMinimax::<MAX_DEPTH,SCORING_ALGORITHM>.
/// MAX_DEPTH : The search depth is now a compile-time constant (const generic)
/// SCORING_ALGORITHM : 
/// 1: alpha-zero material score
/// other: standard material score 
pub struct EngineMinimax<const MAX_DEPTH: usize, const SCORING_ALGORITHM : usize> {
    /// The cloned game state provided during `setup`.
    starting_position: GameState,
    /// Requested calculation time in seconds.
    calculation_time_s: f32,
    /// The instant at which a search was started.
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

/// Implementation of the ChessEngineThreadTrait for EngineMinimax.
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
impl<const MAX_DEPTH: usize, const SCORING_ALGORITHM : usize> ChessEngineThreadTrait for EngineMinimax<MAX_DEPTH,SCORING_ALGORITHM> {
    fn configure(
        &mut self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) {
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

    fn add_string_to_print_log(&mut self, x: &str) {
        self.string_log.push_back(x.to_string());
    }

    fn pop_next_string_to_log(&mut self) -> Option<String> {
        self.string_log.pop_front()
    }

    fn get_calculation_time_as_micros(&self) -> u128 {
        (self.calculation_time_s * 1E6).round() as u128
    }

    /// Pick the best move based on material in the position alone
    fn calculating_callback(&mut self) -> Result<(), ChessErrors> {
        // Use the compile-time depth parameter MAX_DEPTH instead of a runtime value.
        if let Ok(best_move) = self.minimax_top(&self.starting_position.clone(), MAX_DEPTH) {
            self.best_so_far = Some(best_move);
            self.set_status_calculating(false);
        } else {
            return Err(ChessErrors::NoLegalMoves);
        }
        Ok(())
    }
}

impl<const MAX_DEPTH: usize, const SCORING_ALGORITHM : usize> EngineMinimax<MAX_DEPTH,SCORING_ALGORITHM> {
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineMinimax::<MAX_DEPTH,SCORING_ALGORITHM> {
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
            start_time: Instant::now(),
            status_calculating: false,
            best_so_far: None,
            string_log: VecDeque::new()
        }
    }

    /// Recursively evaluates a position using the minimax algorithm with alpha-beta pruning.
    ///
    /// This function applies `move_to_make` to `game`, producing a next_game. It then
    /// either returns a material evaluation at leaf depth or performs a minimax search
    /// with alpha-beta pruning over all legal moves in the next_game. Alpha and beta
    /// are passed and updated down the tree. The maximizing player for a node is
    /// inferred from next_game.turn (Light => maximize, Dark => minimize).
    ///
    /// # Arguments
    /// * `move_to_make` - The candidate move to evaluate (applied immediately).
    /// * `game` - Current game state (move will be applied against this).
    /// * `current_depth` - Current depth in the search tree (1 for a single applied move).
    /// * `max_depth` - Maximum depth to search.
    /// * `alpha` - Current alpha bound (lower bound for maximizing).
    /// * `beta` - Current beta bound (upper bound for minimizing).
    ///
    /// # Returns
    /// * `Ok(Score)` - The evaluated score for this position in the same score convention
    ///                 used by get_material_score() (positive favors Light).
    /// * `Err(ChessErrors)` - If applying moves or generating moves produces an error.
    fn recurse_ab(
        &self,
        move_to_make: CheckedMoveWithFutureGame,
        current_depth: usize,
        max_depth: usize,
        mut alpha: Score,
        mut beta: Score,
    ) -> Result<Score, ChessErrors> {
        if let Some(x) = move_to_make.checked_move.check_status {
            if matches!(x, crate::types_of_check::TypesOfCheck::Checkmate(_, _)) {
                return Ok(generate_losing_score(move_to_make.game_after_move.turn));
            }
        }

        if current_depth == max_depth {
            return match SCORING_ALGORITHM {
                1 => score_game(&move_to_make.game_after_move, ScoringTypes::Material(crate::scoring::MaterialScoringTypes::AlphaZero)),
                _ => score_game(&move_to_make.game_after_move, ScoringTypes::Material(crate::scoring::MaterialScoringTypes::Conventional))
            };
        }

        let exploring_moves = generate_all_moves(&move_to_make.game_after_move)?;
        if exploring_moves.len() == 0 {
            // Not checkmate, so must be stalemate
            return Ok(0.0);
        }

        // Determine whether the player to move in next_game is maximizing (Light) or minimizing (Dark)
        let is_maximizing = matches!(move_to_make.game_after_move.turn, PieceTeam::Light);

        if is_maximizing {
            let mut value = MIN_SCORE;
            for mv in exploring_moves.into_iter() {
                let child = self.recurse_ab(mv, current_depth + 1, max_depth, alpha, beta)?;
                if child > value {
                    value = child;
                }
                if value > alpha {
                    alpha = value;
                }
                // Beta cutoff
                if alpha >= beta {
                    break;
                }
            }
            Ok(value)
        } else {
            let mut value = MAX_SCORE;
            for mv in exploring_moves.into_iter() {
                let child = self.recurse_ab(mv, current_depth + 1, max_depth, alpha, beta)?;
                if child < value {
                    value = child;
                }
                if value < beta {
                    beta = value;
                }
                // Alpha cutoff
                if beta <= alpha {
                    break;
                }
            }
            Ok(value)
        }
    }

    /// Performs a minimax search with alpha-beta pruning from the root position to find the best move.
    ///
    /// Generates all legal moves at the root and evaluates each one via recurse_ab using
    /// full alpha/beta window [MIN_SCORE, MAX_SCORE]. The root decision selects the move
    /// yielding the numerically largest score when Light to move, or numerically smallest when Dark to move.
    fn minimax_top(&self, game: &GameState, max_depth: usize) -> Result<MoveDescription, ChessErrors> {
        let exploring_moves = generate_all_moves(game)?;
        if exploring_moves.len() == 0 {
            return Err(ChessErrors::NoLegalMoves);
        }

        // Evaluate the first move to initialize best score/move
        let first_move = exploring_moves.front().unwrap().clone();
        let first_score = self.recurse_ab(first_move.clone(), 1, max_depth, MIN_SCORE, MAX_SCORE)?;
        let mut best_move_so_far = first_move;
        let mut best_score_so_far = first_score;

        let is_root_maximizing = matches!(game.turn, PieceTeam::Light);

        for mv in exploring_moves.into_iter().skip(1) {
            let score = self.recurse_ab(mv.clone(), 1, max_depth, MIN_SCORE, MAX_SCORE)?;
            if (is_root_maximizing && score > best_score_so_far)
                || (!is_root_maximizing && score < best_score_so_far)
            {
                best_score_so_far = score;
                best_move_so_far = mv;
            }
        }

        Ok(best_move_so_far.checked_move.description)
    }
}