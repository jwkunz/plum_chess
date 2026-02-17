use crate::game_state::{chess_types::*, game_state::GameState};

pub const ALL_PIECE_KINDS: [PieceKind; 6] = [
    PieceKind::Pawn,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Rook,
    PieceKind::Queen,
    PieceKind::King,
];

#[inline]
pub fn piece_on_square_for_color(
    game_state: &GameState,
    color: Color,
    square: Square,
) -> Option<PieceKind> {
    let mask = 1u64 << square;
    for piece in ALL_PIECE_KINDS {
        if (game_state.pieces[color.index()][piece.index()] & mask) != 0 {
            return Some(piece);
        }
    }
    None
}

#[inline]
pub fn enemy_piece_on(game_state: &GameState, square: Square) -> Option<PieceKind> {
    piece_on_square_for_color(game_state, game_state.side_to_move.opposite(), square)
}

#[inline]
pub fn piece_on_square_any(game_state: &GameState, square: Square) -> Option<(Color, PieceKind)> {
    if let Some(piece) = piece_on_square_for_color(game_state, Color::Light, square) {
        return Some((Color::Light, piece));
    }
    if let Some(piece) = piece_on_square_for_color(game_state, Color::Dark, square) {
        return Some((Color::Dark, piece));
    }
    None
}
