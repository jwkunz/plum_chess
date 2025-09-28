use std::{collections::{BTreeMap, VecDeque}, sync::mpsc, time::Instant};

use crate::{
    chess_engine_thread_trait::{
        ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType,
    },
    chess_move::{self, ChessMove},
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
            if let Some(search_result) = self.find_best_move(&self.starting_position.clone(),false) {
                self.best_so_far = Some(search_result.chess_move);
                self.add_string_to_print_log("Finishing Engine Search".into());
            } else {
                self.add_string_to_print_log("Found no viable move");
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
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

    fn score_game(&mut self, starting_position: &GameState) -> f32{
        starting_position.get_material_score() as f32
    }

    fn find_best_move(&mut self, starting_position: &GameState, terminal_search_flag: bool) -> Option<BestMoveSearchResult> {
        let mut result = None;
        // Generate all possible moves
        if let Ok(all_moves) = generate_all_moves(starting_position) {
            // Skip if no available moves
            if all_moves.len() == 0{
                return result;
            }
            // Make room to store scores
            let mut all_move_scores = Vec::<BestMoveSearchResult>::with_capacity(all_moves.len());
            for chess_move in all_moves {
                // If the move can be done
                if let Ok(game_after_move) = apply_move_to_game(starting_position, &chess_move.description)
                {
                    if terminal_search_flag{
                        // Score this move
                        all_move_scores.push(BestMoveSearchResult { chess_move: chess_move.description, score: self.score_game(&game_after_move)}); 
                    }else{
                    // Find the best move the opponent can make
                        if let Some(best_opponent_move) = self.find_best_move(&game_after_move, true){
                            all_move_scores.push(BestMoveSearchResult { chess_move: chess_move.description, score: best_opponent_move.score }); 
                        }
                    }
                }
            }
            // Change direction based on color to enforce signed-ness
            let direction : f32 = match starting_position.turn {
                crate::piece_types::PieceTeam::Dark=>{-1.0},
                crate::piece_types::PieceTeam::Light=>{1.0}
            };
            // Default to take the first move
            result = Some(all_move_scores.first().unwrap().clone());
            // Unless can find something better
            for m in all_move_scores.iter().skip(1){
                // Do this comparison with the same signed-ness
                if m.score*direction >= result.as_ref().unwrap().score*direction{
                    result = Some(m.clone())
                }
            }
        }
        result
    }
}
