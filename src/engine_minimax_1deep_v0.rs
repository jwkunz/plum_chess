use std::{collections::VecDeque, sync::mpsc, time::Instant};

use crate::{
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_move::ChessMove,
    errors::Errors,
    game_state::GameState,
    move_logic::{apply_move_to_game, generate_all_moves},
};

/// This engine simply looks at the next moves and picks the one that maximizes the conventional score on the next turn
/// It has no strategy from the opponent
pub struct EngineMinimax1DeepV0 {
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

impl ChessEngineThreadTrait for EngineMinimax1DeepV0 {
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

    fn add_string_to_print_log(&mut self, x: &str) {
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
            self.add_string_to_print_log("Starting Engine Search".into());
            if let Some(search_result) = self.minimax_1_layer(&self.starting_position.clone()) {
                self.best_so_far = Some(search_result.chess_move);
                self.add_string_to_print_log("Finishing Engine Search".into());
            } else {
                self.add_string_to_print_log("Found no viable move");
            }
        }
        Ok(())
    }
}

struct BestMoveSearchResult {
    chess_move: ChessMove,
    score: f32,
}
impl EngineMinimax1DeepV0 {
    pub fn new(
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Self {
        EngineMinimax1DeepV0 {
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

    /// Helper function to find the best move
    fn find_best_move(&mut self, starting_position: &GameState) -> Option<BestMoveSearchResult> {
        let mut result = None;
        let mut best_score_a: f32 = starting_position.get_material_score() as f32;
        // Generate all possible moves
        if let Ok(moves_a) = generate_all_moves(starting_position) {
            for chess_move_a in moves_a {
                // If the move can be done
                if let Ok(game_a) = apply_move_to_game(starting_position, &chess_move_a.description)
                {
                    // Always pick at least one move if there is one available
                    if result.is_none() {
                        best_score_a = game_a.get_material_score() as f32;
                        result = Some(BestMoveSearchResult {
                            chess_move: chess_move_a.description.clone(),
                            score: best_score_a as f32,
                        });
                    } else {
                        // Get the conventional material score
                        let layer_score_a = game_a.get_material_score() as f32;
                        // Is this a higher score
                        let mut improvement = false;
                        // Score direction
                        match starting_position.turn {
                            crate::piece_types::PieceTeam::Dark => {
                                improvement = layer_score_a <= best_score_a
                            }
                            crate::piece_types::PieceTeam::Light => {
                                improvement = layer_score_a >= best_score_a
                            }
                        }
                        // Keep the best
                        if improvement {
                            best_score_a = layer_score_a;
                            result = Some(BestMoveSearchResult {
                                chess_move: chess_move_a.description.clone(),
                                score: layer_score_a as f32,
                            });
                        }
                    }
                }
            }
        }
        result
    }

    fn minimax_1_layer(&mut self, starting_position: &GameState) -> Option<BestMoveSearchResult> {
        let mut result = None;
        let mut best_score_b: f32 = starting_position.get_material_score() as f32;
        // Generate all possible moves
        if let Ok(moves_a) = generate_all_moves(starting_position) {
            for chess_move_a in moves_a {
                // If the move can be done
                if let Ok(game_a) = apply_move_to_game(starting_position, &chess_move_a.description)
                {
                    // Always pick at least one move if there is one available
                    if result.is_none() {
                        best_score_b = game_a.get_material_score() as f32;
                        result = Some(BestMoveSearchResult {
                            chess_move: chess_move_a.description.clone(),
                            score: best_score_b as f32,
                        });
                    } else {
                        if let Some(best_move_b_result) = self.find_best_move(&game_a) {
                            // Is this a better score
                            let mut improvement = false;
                            // Seeking worst opponent score
                            match starting_position.turn {
                                crate::piece_types::PieceTeam::Dark => {
                                    improvement = best_move_b_result.score <= best_score_b
                                }
                                crate::piece_types::PieceTeam::Light => {
                                    improvement = best_move_b_result.score >= best_score_b
                                }
                            }
                            // Keep the best
                            if improvement {
                                best_score_b = best_move_b_result.score;
                                result = Some(BestMoveSearchResult {
                                    chess_move: chess_move_a.description.clone(),
                                    score: best_score_b as f32,
                                });
                            }
                        }
                    }
                }
            }
        }
        result
    }
}
