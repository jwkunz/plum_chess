use std::collections::LinkedList;
use crate::{
    board_location::BoardLocation, board_mask::BoardMask, checked_move_description::CheckedMoveDescription, chess_errors::ChessErrors, game_state::GameState, generate_movements::*, move_description::MoveDescription, piece_class::PieceClass, piece_record::PieceRecord, piece_register::{self, PieceRegister}, piece_team::PieceTeam
};
type ListOfUncheckedMoves = LinkedList<MoveDescription>;
type ListOfCheckedMoves = LinkedList<CheckedMoveDescription>;