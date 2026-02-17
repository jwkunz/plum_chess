pub mod game_state{
    pub mod chess_types;
    pub mod game_state;
    pub mod undo_state;
    pub mod chess_rules;
}

pub mod moves{
    pub mod bishop_moves;
    pub mod king_moves;
    pub mod knight_moves;
    pub mod pawn_moves;
    pub mod queen_moves;
    pub mod rook_moves;
    pub mod move_descriptions;
}

pub mod move_generation{
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

pub mod utils {
    pub mod fen_generator;
    pub mod fen_parser;
    pub mod algebraic;
    pub mod long_algebraic;
    pub mod render_game_state;
}
