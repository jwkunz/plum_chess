//! Core incremental board state representation.
//!
//! `GameState` is the central model for the engine. It stores piece bitboards,
//! occupancy caches, turn/state flags, clocks, and history stacks used by
//! make/unmake style workflows and higher-level engine systems.

use crate::game_state::chess_rules::STARTING_POSITION_FEN;
use crate::game_state::chess_types::*;
use crate::utils::fen_generator::generate_fen;
use crate::utils::fen_parser::parse_fen;

/// Incremental game state optimized for fast move making/unmaking.
#[derive(Debug, Clone)]
pub struct GameState {
    // --- Bitboard representation ---
    // [color][piece_kind]
    pub pieces: [[u64; 6]; 2],

    // Occupancy caches.
    pub occupancy_by_color: [u64; 2],
    pub occupancy_all: u64,

    // --- Side and state flags ---
    pub side_to_move: Color,
    pub castling_rights: CastlingRights,
    pub en_passant_square: Option<Square>,

    // --- Clocks / move counters ---
    pub halfmove_clock: u16,
    pub fullmove_number: u16,

    // --- Incremental hashing ---
    pub zobrist_key: u64,
    pub pawn_zobrist_key: u64,

    // --- Search / repetition support ---
    pub ply: u16,
    pub repetition_history: Vec<u64>,

    // --- Make/unmake stack ---
    pub undo_stack: Vec<UndoState>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            pieces: [[0; 6]; 2],
            occupancy_by_color: [0; 2],
            occupancy_all: 0,

            side_to_move: Color::Light,
            castling_rights: 0,
            en_passant_square: None,

            halfmove_clock: 0,
            fullmove_number: 1,

            zobrist_key: 0,
            pawn_zobrist_key: 0,

            ply: 0,
            repetition_history: Vec::new(),
            undo_stack: Vec::new(),
        }
    }
}

impl GameState {
    /// Placeholder constructor for now. Later this can parse FEN or initialize startpos.
    #[inline]
    pub fn new_empty() -> Self {
        Self::default()
    }

    #[inline]
    pub fn new_game() -> Self {
        parse_fen(STARTING_POSITION_FEN).expect("starting FEN should always parse")
    }

    #[inline]
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        parse_fen(fen)
    }

    #[inline]
    pub fn get_fen(&self) -> String {
        generate_fen(self)
    }
}
