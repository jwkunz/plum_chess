use std::{collections::VecDeque, sync::mpsc, time::Instant};

use crate::{
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_move::ChessMove,
    errors::Errors,
    game_state::GameState,
    move_logic::{apply_move_to_game, generate_all_moves}
};

/// This engine simply looks at the next moves and picks the one that maximizes the conventional score on the next turn
/// It has no strategy from the opponent
pub struct EngineGreedy1Move {
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
    string_log: VecDeque<String>,
    /// IO
    command_receiver: mpsc::Receiver<EngineControlMessageType>,
    response_sender: mpsc::Sender<EngineResponseMessageType>,
}

impl ChessEngineThreadTrait for EngineGreedy1Move {
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

    fn get_best_move_so_far(&self) -> Option<ChessMove> {
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

    fn calculating_callback(&mut self) -> Result<(), Errors> {
        // If nothing has been calculated
        if self.best_so_far.is_none() {
            // Generate all possible moves
            if let Ok(moves) = generate_all_moves(&self.starting_position) {
                // Search for the best
                let mut best_score: i8 = 0;
                for chess_move in moves {
                    // If the move can be done
                    if let Ok(trial_game) =
                        apply_move_to_game(&self.starting_position, &chess_move.description)
                    {
                        // Get the conventional material score
                        let layer_1_score = trial_game.get_material_score();
                        // Is this a higher score
                        let mut improvement = false;
                        match self.starting_position.turn{
                            crate::piece_types::PieceTeam::Dark => {improvement = layer_1_score <= best_score},
                            crate::piece_types::PieceTeam::Light => {improvement = layer_1_score >= best_score},
                        }
                        // Keep the best                            
                        if improvement{
                            self.add_string_to_print_log(&format!(
                                "Found new best candidate move: {:?} with score {:?}",
                                chess_move.description, layer_1_score
                            ));
                            best_score = layer_1_score;
                            self.best_so_far = Some(chess_move.description);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl EngineGreedy1Move {
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineGreedy1Move {
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
