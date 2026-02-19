//! Crate root module declarations for the Plum Chess engine project.
//!
//! This file exposes all top-level subsystems (game state, move generation,
//! search, engines, UCI protocol handling, and utility helpers) so binaries,
//! tests, and external tooling can import stable module paths.

pub mod game_state {
    pub mod chess_rules;
    pub mod chess_types;
    pub mod game_state;
    pub mod undo_state;
}

pub mod moves {
    pub mod bishop_moves;
    pub mod king_moves;
    pub mod knight_moves;
    pub mod move_descriptions;
    pub mod pawn_moves;
    pub mod queen_moves;
    pub mod rook_moves;
}

pub mod move_generation {
    pub mod legal_move_apply;
    pub mod legal_move_checks;
    pub mod legal_move_generator;
    pub mod legal_move_shared;
    pub mod legal_moves_bishop;
    pub mod legal_moves_king;
    pub mod legal_moves_knight;
    pub mod legal_moves_pawn;
    pub mod legal_moves_queen;
    pub mod legal_moves_rook;
    pub mod move_generator;
    pub mod perft;
}

pub mod search {
    pub mod board_scoring;
    pub mod iterative_deepening;
    pub mod iterative_deepening_v3;
    pub mod iterative_deepening_v4;
    pub mod iterative_deepening_v5;
    pub mod iterative_deepening_v6;
    pub mod iterative_deepening_v7;
    pub mod iterative_deepening_v8;
    pub mod transposition_table;
    pub mod zobrist;
}
pub mod tables {
    pub mod opening_book;
}
pub mod uci {
    pub mod uci_top;
}
pub mod engines {
    pub mod engine_greedy;
    pub mod engine_iterative_v1;
    pub mod engine_iterative_v2;
    pub mod engine_iterative_v3;
    pub mod engine_iterative_v4;
    pub mod engine_iterative_v5;
    pub mod engine_iterative_v6;
    pub mod engine_iterative_v7;
    pub mod engine_iterative_v8;
    pub mod engine_random;
    pub mod engine_trait;
}

pub mod utils {
    pub mod algebraic;
    pub mod engine_match_harness;
    pub mod fen_generator;
    pub mod fen_parser;
    pub mod long_algebraic;
    pub mod pgn;
    pub mod render_game_state;
}
