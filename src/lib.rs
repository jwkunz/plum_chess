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

pub mod utils {
    pub mod fen_generator;
    pub mod fen_parser;
    pub mod algebraic;
    pub mod render_game_state;
}
