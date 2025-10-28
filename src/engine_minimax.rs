use std::{collections::VecDeque, sync::mpsc, time::Instant};

use crate::{
    apply_move_to_game::apply_move_to_game_unchecked,
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_errors::ChessErrors,
    game_state::{self, GameState},
    generate_all_moves::generate_all_moves,
    move_description::MoveDescription,
    piece_team::PieceTeam,
    scoring::{self, compare_scores, generate_losing_score, Score},
};

pub struct EngineMinimax {
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
impl ChessEngineThreadTrait for EngineMinimax {
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
        if let Ok(best_move) = minimax_top(&self.starting_position.clone(), 3) {
            self.best_so_far = Some(best_move);
            self.set_status_calculating(false);
        } else {
            return Err(ChessErrors::NoLegalMoves);
        }
        Ok(())
    }
}

impl EngineMinimax {
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineMinimax {
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


fn recurse(
    move_to_make: MoveDescription,
    game: &GameState,
    direction_flipper: Score,
    minimax_flipper : Score,
    current_depth: usize,
    max_depth: usize,
) -> Result<Score, ChessErrors> {
    let next_game = apply_move_to_game_unchecked(&move_to_make, game)?;
    if current_depth == max_depth {
        return Ok(next_game.get_material_score());
    } else {
        let exploring_moves = generate_all_moves(&next_game)?;
        if exploring_moves.len() == 0 {
            return Ok(generate_losing_score(next_game.turn));
        }
        let mut best_so_far = recurse(
            exploring_moves
                .front()
                .unwrap()
                .checked_move
                .description
                .clone(),
            &next_game,
            direction_flipper,
            -minimax_flipper,
            current_depth + 1,
            max_depth,
        )?;
        let flip = direction_flipper*minimax_flipper;
        for i in exploring_moves.into_iter().skip(1) {
            let branch_result = recurse(
                i.checked_move.description,
                &next_game,
                direction_flipper,
                -minimax_flipper,
                current_depth + 1,
                max_depth,
            )?;
            if branch_result*flip > best_so_far*flip{
                best_so_far = branch_result;
            }
        }
        Ok(best_so_far)
    }
}

fn minimax_top(
    game: &GameState,
    max_depth: usize,
) -> Result<MoveDescription, ChessErrors> {
    let exploring_moves = generate_all_moves(game)?;
    if exploring_moves.len() == 0 {
        return Err(ChessErrors::NoLegalMoves);
    }
    let direction_flipper = match game.turn {
        PieceTeam::Light => 1.0,
        PieceTeam::Dark => -1.0
    };
    let minimax_flipper = 1.0;
    let mut best_move_so_far =exploring_moves
            .front()
            .unwrap()
            .checked_move
            .description
            .clone();

    let mut best_score_so_far = recurse(
        best_move_so_far.clone(),
        &game,
        direction_flipper,
        -minimax_flipper,
        1,
        max_depth,
    )?;
    let flip = direction_flipper*minimax_flipper;
    for i in exploring_moves.into_iter().skip(1) {
        let branch_result =
            recurse(i.checked_move.description.clone(), &game, direction_flipper,-minimax_flipper,1, max_depth)?;
        if branch_result*flip > best_score_so_far*flip{
            best_score_so_far = branch_result;
            best_move_so_far = i.checked_move.description;
        }
    }
    Ok(best_move_so_far)
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test_minimax(){
        let game = GameState::from_fen("7k/8/8/8/6r1/5P2/8/7K w - - 0 1").unwrap();
        let result = minimax_top(&game, 4).unwrap();
        dbg!(result);

    }
}