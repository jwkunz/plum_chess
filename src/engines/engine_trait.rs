//! Engine abstraction layer used by the UCI subsystem.
//!
//! Defines common input parameters and output payloads so different engine
//! strategies can be selected at runtime behind a single trait interface.

use crate::game_state::game_state::GameState;

#[derive(Debug, Clone, Default)]
pub struct GoParams {
    pub depth: Option<u8>,
    pub movetime_ms: Option<u64>,
    pub wtime_ms: Option<u64>,
    pub btime_ms: Option<u64>,
    pub winc_ms: Option<u64>,
    pub binc_ms: Option<u64>,
    pub movestogo: Option<u16>,
    pub searchmoves: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineOutput {
    pub best_move: Option<u64>,
    pub info_lines: Vec<String>,
}

pub trait Engine: Send {
    fn new_game(&mut self) {}
    fn set_option(&mut self, _name: &str, _value: &str) -> Result<(), String> {
        Ok(())
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String>;
}
