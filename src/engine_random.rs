use std::{sync::mpsc, thread::{self, JoinHandle, Thread}, time::{Duration, Instant}};

use rand::{seq::IteratorRandom, thread_rng};

use crate::{
    chess_engine_thread_trait::ChessEngineThreadTrait, chess_move::ChessMove, errors::Errors, game_state::GameState, move_logic::generate_all_moves
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
struct EngineRandomWorker {
    /// The cloned game state provided during `setup`. None until setup is called.
    starting_position: GameState,
    /// Requested calculation time in seconds. None until setup is called.
    calculation_time_s: Option<f32>,
    /// The instant at which a search was started. Used to emulate timing behavior.
    start_time: Instant,
    /// Calculation status
    calculating_status : bool,
    /// Best move so far
    best_so_far : Option<ChessMove>,
    /// IO
    command_receiver : mpsc::Receiver<EngineControlMessageType>,
    response_sender : mpsc::Sender<EngineResponseMessageType>
}

enum EngineControlMessageType{
    StartCalculating,
    AreYouStillCalculating,
    GiveMeYourBestMoveSoFar,
    StopNow,
}
enum EngineResponseMessageType{
    BestMoveFound(Option<ChessMove>),
    HadAnError(Errors),
    StillCalculatingStatus(bool)
}

impl EngineRandomWorker{
    fn new(
        starting_position: GameState,
        calculation_time_s: Option<f32>, 
        command_receiver : mpsc::Receiver<EngineControlMessageType>,  
        response_sender : mpsc::Sender<EngineResponseMessageType>) -> Self{
        EngineRandomWorker { starting_position, calculation_time_s, start_time: Instant::now(), calculating_status: false, best_so_far:None, command_receiver, response_sender}
    }
    fn tick(&mut self){
        let mut message_in = None;
        if let Ok(x) = self.command_receiver.try_recv(){
            message_in = Some(x);
        }

        match message_in{
            Some(EngineControlMessageType::StartCalculating)=>{
                self.calculating_status = true;
                self.start_time = Instant::now();
            }
            Some(EngineControlMessageType::AreYouStillCalculating) =>{
                let _ = self.response_sender.send(EngineResponseMessageType::StillCalculatingStatus(self.calculating_status));
            }
            Some(EngineControlMessageType::StopNow) => {
                self.calculating_status = false;
            },
            Some(EngineControlMessageType::GiveMeYourBestMoveSoFar) => {
                let _ = self.response_sender.send(EngineResponseMessageType::BestMoveFound(self.best_so_far.clone()));
            }
            _ => () // Ignore others
        }

        // Handle timeout
        if let Some(limit) = self.calculation_time_s{
            let elapsed_time : Duration = Instant::now() - self.start_time;
            if elapsed_time.as_millis() >= (limit*1E6).round() as u128{
                self.calculating_status = false;
            }
        }

        // Do calculation activities here
        if self.calculating_status {
            if let Ok(moves) = generate_all_moves(&self.starting_position) {
                let mut rng = thread_rng();
                if let Some(random_move) = moves.iter().choose(&mut rng) {
                    self.best_so_far = Some(random_move.description.clone());
                    // Done calculating
                    self.calculating_status = false;
                }
            }
        }
    }
}

pub struct EngineRandom{
    /// Thread IO
    command_sender : mpsc::Sender<EngineControlMessageType>,
    response_receiver : mpsc::Receiver<EngineResponseMessageType>
}

impl ChessEngineThreadTrait for EngineRandom {
    
    /// Prepare the engine for a calculation.
    ///
    /// The engine clones and stores the provided `game` state and records the requested
    /// calculation time in seconds. The engine resets any previous search result and
    /// clears the `done_searching` flag so a subsequent `start_searching` will perform work.
    ///
    /// This method is synchronous and lightweight.
    fn new(game: &crate::game_state::GameState, calculation_time_s: f32) -> Self {
        let (command_sender,command_receiver) = mpsc::channel::<EngineControlMessageType>();
        let (response_sender, response_receiver)  = mpsc::channel::<EngineResponseMessageType>();
        let mut engine = EngineRandomWorker::new(game.clone(), Some(calculation_time_s), command_receiver, response_sender);
        thread::spawn(move ||{ 
            loop{
            engine.tick();
            }
        });
        EngineRandom{
            command_sender,
            response_receiver
        }
    }
    
    /// Signal the engine to stop searching.
    ///
    /// For this simple engine this sets the `done_searching` flag. If a real search loop
    /// existed it would use this hint to interrupt work and return quickly.
    fn stop_searching(&mut self) {
        let _ = self.command_sender.send(EngineControlMessageType::StopNow);
        thread::sleep(Duration::from_millis(10));
    }
    
    /// Return whether the engine has finished its current search.
    ///
    /// For EngineRandom this is a simple boolean flag toggled by `start_searching` or
    /// `stop_searching`.
    fn is_done_searching(&self) -> bool {
        let send_status = self.command_sender.send(EngineControlMessageType::AreYouStillCalculating);
        thread::sleep(Duration::from_millis(10));
        if let Ok(x) = self.response_receiver.recv(){
            match x {
                EngineResponseMessageType::StillCalculatingStatus(y) => !y,
                _ => false
            }
        }else{
            false
        }
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
        let _ = self.command_sender.send(EngineControlMessageType::StartCalculating);
        thread::sleep(Duration::from_millis(10));
    }

    /// Return the best move found by the last search, if any.
    ///
    /// The returned `ChessMove` is cloned from the internal storage. `None` indicates that
    /// no move was selected (for example, if no legal moves exist or `setup` was not called).
    fn get_best_move(&self) -> Option<ChessMove> {
        let _ = self.command_sender.send(EngineControlMessageType::GiveMeYourBestMoveSoFar);
        thread::sleep(Duration::from_millis(10));
        if let Ok(x) = self.response_receiver.recv(){
            match x {
                EngineResponseMessageType::BestMoveFound(y) => y,
                _ => None
            }
        }else{
            None
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_random_engine(){
        let new_game = GameState::new_game();
        let mut dut = EngineRandom::new(&new_game,1.0);
        dut.start_searching();
        while dut.is_done_searching() == false{
            thread::sleep(Duration::from_millis(10));
        }
        dbg!(dut.get_best_move());
    }
}
