use std::{sync::mpsc, thread::sleep, time::Duration};

use crate::{chess_errors::ChessErrors, game_state::GameState, move_description::MoveDescription};

#[derive(Debug)]
pub enum EngineControlMessageType{
    StartCalculating,
    AreYouStillCalculating,
    GiveMeYourBestMoveSoFar,
    GiveMeAStringToLog,
    StopNow,
}
#[derive(Debug)]
pub enum EngineResponseMessageType{
    BestMoveFound(Option<MoveDescription>),
    HadAnError(ChessErrors),
    StillCalculatingStatus(bool),
    StringToLog(Option<String>),
}


/// Trait describing the minimal engine thread interface expected by the UCI handler.
///
/// Implementors encapsulate an engine instance that can be prepared with a position,
/// started and stopped for a search, and queried for completion and the best move.
/// The trait is intentionally small to allow both concrete and boxed dynamic usage:
/// - `new` is constrained to `Sized` so the trait remains object-safe for the other methods.
/// - All other methods use `&self`/`&mut self` without generic parameters.
///
/// Typical usage:
/// - Construct a concrete implementor (e.g., `EngineRandom::new()`).
/// - Call `setup` with a `GameState` and desired calculation time.
/// - Call `start_searching` (may be synchronous or spawn a background thread).
/// - Periodically call `is_done_searching` or let the UCI handler poll the engine.
/// - Retrieve the result with `get_best_move`.
/// 
pub trait ChessEngineThreadTrait : Send {

    fn configure(
        &mut self,
        starting_position: GameState, 
        calculation_time_s: f32, 
        command_receiver : mpsc::Receiver<EngineControlMessageType>, 
        response_sender : mpsc::Sender<EngineResponseMessageType>);
    
    fn record_start_time(&mut self);

    fn compute_elapsed_micros(&self) -> u128;

    fn set_status_calculating(&mut self, x : bool);

    fn get_status_calculating(&self) -> bool;

    fn get_command_receiver(&self) -> &mpsc::Receiver<EngineControlMessageType>;

    fn get_response_sender(&self) -> &mpsc::Sender<EngineResponseMessageType>;

    fn get_best_move_so_far(&self) -> Option<MoveDescription>;

    fn pop_next_string_to_log(&mut self) -> Option<String>;

    fn add_string_to_print_log(&mut self, x : &str);

    fn get_calculation_time_as_micros(&self) -> u128;

    /// The engine is designed to operate in polling time intervals, monitoring message traffic like so
    fn tick(&mut self){
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

    /// This function is where you put the chess computing logic, called in repeated intervals
    fn calculating_callback(&mut self) -> Result<(), ChessErrors>;

}