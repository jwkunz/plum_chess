//! Terminal-oriented Unicode board renderer.
//!
//! Creates a human-readable board view from internal bitboards for debugging,
//! tests, and diagnostics in text environments.

use crate::game_state::{chess_types::*, game_state::GameState};

/// Render the board to a Unicode string for terminal output.
///
/// Assumes square indexing where `0 == a1`, `7 == h1`, and `63 == h8`.
pub fn render_game_state(game_state: &GameState) -> String {
    let mut out = String::new();

    out.push_str("  a b c d e f g h\n");

    for rank in (0..8).rev() {
        out.push(char::from(b'1' + rank as u8));
        out.push(' ');

        for file in 0..8 {
            let sq = rank * 8 + file;
            match piece_on_square(game_state, sq) {
                Some(ch) => out.push(ch),
                None => out.push('·'),
            }

            if file < 7 {
                out.push(' ');
            }
        }

        out.push(' ');
        out.push(char::from(b'1' + rank as u8));
        out.push('\n');
    }

    out.push_str("  a b c d e f g h");

    out
}

fn piece_on_square(game_state: &GameState, square: usize) -> Option<char> {
    let mask = 1u64 << square;

    for color in [Color::Light, Color::Dark] {
        let color_idx = color.index();

        for piece in [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ] {
            if (game_state.pieces[color_idx][piece.index()] & mask) != 0 {
                return Some(piece_to_unicode(color, piece));
            }
        }
    }

    None
}

fn piece_to_unicode(color: Color, piece: PieceKind) -> char {
    match (color, piece) {
        (Color::Light, PieceKind::Pawn) => '♙',
        (Color::Light, PieceKind::Knight) => '♘',
        (Color::Light, PieceKind::Bishop) => '♗',
        (Color::Light, PieceKind::Rook) => '♖',
        (Color::Light, PieceKind::Queen) => '♕',
        (Color::Light, PieceKind::King) => '♔',
        (Color::Dark, PieceKind::Pawn) => '♟',
        (Color::Dark, PieceKind::Knight) => '♞',
        (Color::Dark, PieceKind::Bishop) => '♝',
        (Color::Dark, PieceKind::Rook) => '♜',
        (Color::Dark, PieceKind::Queen) => '♛',
        (Color::Dark, PieceKind::King) => '♚',
    }
}
