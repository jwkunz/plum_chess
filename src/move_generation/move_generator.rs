use std::error::Error;
use std::fmt;

use crate::game_state::game_state::GameState;

pub type MoveGenResult<T> = Result<T, MoveGenerationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveGenerationError {
    NotImplemented,
    InvalidState(String),
}

impl fmt::Display for MoveGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveGenerationError::NotImplemented => {
                write!(f, "move generation is not implemented yet")
            }
            MoveGenerationError::InvalidState(msg) => write!(f, "invalid game state: {msg}"),
        }
    }
}

impl Error for MoveGenerationError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoveAnnotations {
    pub gives_check: bool,
    pub is_discovery_check: bool,
    pub is_double_check: bool,
    pub is_checkmate: bool,
}

#[derive(Debug, Clone)]
pub struct GeneratedMove {
    pub move_description: u64,
    pub game_after_move: GameState,
    pub annotations: MoveAnnotations,
}

pub trait MoveGenerator: Send + Sync {
    fn generate_legal_moves(&self, game_state: &GameState) -> MoveGenResult<Vec<GeneratedMove>>;
}

pub struct NullMoveGenerator;

impl MoveGenerator for NullMoveGenerator {
    fn generate_legal_moves(&self, _game_state: &GameState) -> MoveGenResult<Vec<GeneratedMove>> {
        Err(MoveGenerationError::NotImplemented)
    }
}
