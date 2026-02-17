use crate::game_state::{chess_types::*, game_state::GameState};
use crate::utils::algebraic::square_to_algebraic;

pub fn generate_fen(game_state: &GameState) -> String {
    let board = generate_board_field(game_state);
    let side_to_move = match game_state.side_to_move {
        Color::Light => "w",
        Color::Dark => "b",
    };
    let castling = generate_castling_field(game_state.castling_rights);
    let en_passant = generate_en_passant_field(game_state.en_passant_square);

    format!(
        "{} {} {} {} {} {}",
        board,
        side_to_move,
        castling,
        en_passant,
        game_state.halfmove_clock,
        game_state.fullmove_number
    )
}

fn generate_board_field(game_state: &GameState) -> String {
    let mut out = String::new();

    for rank in (0..8).rev() {
        let mut empty_count = 0u8;

        for file in 0..8 {
            let sq = rank * 8 + file;
            if let Some(ch) = piece_fen_char_on_square(game_state, sq) {
                if empty_count > 0 {
                    out.push(char::from(b'0' + empty_count));
                    empty_count = 0;
                }
                out.push(ch);
            } else {
                empty_count += 1;
            }
        }

        if empty_count > 0 {
            out.push(char::from(b'0' + empty_count));
        }

        if rank > 0 {
            out.push('/');
        }
    }

    out
}

fn piece_fen_char_on_square(game_state: &GameState, square: usize) -> Option<char> {
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
                return Some(piece_to_fen_char(color, piece));
            }
        }
    }

    None
}

fn piece_to_fen_char(color: Color, piece: PieceKind) -> char {
    let base = match piece {
        PieceKind::Pawn => 'p',
        PieceKind::Knight => 'n',
        PieceKind::Bishop => 'b',
        PieceKind::Rook => 'r',
        PieceKind::Queen => 'q',
        PieceKind::King => 'k',
    };

    match color {
        Color::Light => base.to_ascii_uppercase(),
        Color::Dark => base,
    }
}

fn generate_castling_field(rights: CastlingRights) -> String {
    let mut out = String::new();

    if (rights & CASTLE_LIGHT_KINGSIDE) != 0 {
        out.push('K');
    }
    if (rights & CASTLE_LIGHT_QUEENSIDE) != 0 {
        out.push('Q');
    }
    if (rights & CASTLE_DARK_KINGSIDE) != 0 {
        out.push('k');
    }
    if (rights & CASTLE_DARK_QUEENSIDE) != 0 {
        out.push('q');
    }

    if out.is_empty() {
        out.push('-');
    }

    out
}

fn generate_en_passant_field(square: Option<Square>) -> String {
    let Some(square) = square else {
        return "-".to_owned();
    };

    square_to_algebraic(square).unwrap_or_else(|_| "-".to_owned())
}

#[cfg(test)]
mod tests {
    use super::generate_fen;
    use crate::game_state::chess_rules::STARTING_POSITION_FEN;
    use crate::game_state::chess_types::{
        CASTLE_DARK_KINGSIDE, CASTLE_DARK_QUEENSIDE, CASTLE_LIGHT_KINGSIDE,
        CASTLE_LIGHT_QUEENSIDE, Color,
    };
    use crate::utils::fen_parser::parse_fen;

    #[test]
    fn round_trip_starting_position_fen() {
        let parsed = parse_fen(STARTING_POSITION_FEN).expect("starting FEN should parse");
        let generated = generate_fen(&parsed);

        assert_eq!(generated, STARTING_POSITION_FEN);

        let reparsed = parse_fen(&generated).expect("generated FEN should parse");
        assert_eq!(reparsed.pieces, parsed.pieces);
        assert_eq!(reparsed.side_to_move, parsed.side_to_move);
        assert_eq!(reparsed.castling_rights, parsed.castling_rights);
        assert_eq!(reparsed.en_passant_square, parsed.en_passant_square);
        assert_eq!(reparsed.halfmove_clock, parsed.halfmove_clock);
        assert_eq!(reparsed.fullmove_number, parsed.fullmove_number);
    }

    #[test]
    fn round_trip_custom_position_fen() {
        let fen = "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/2N2N2/PPPP1PPP/R1BQ1RK1 b kq - 4 6";
        let parsed = parse_fen(fen).expect("custom FEN should parse");
        let generated = generate_fen(&parsed);
        let reparsed = parse_fen(&generated).expect("generated FEN should parse");

        assert_eq!(generated, fen);
        assert_eq!(reparsed.pieces, parsed.pieces);
        assert_eq!(reparsed.side_to_move, Color::Dark);
        assert_eq!(
            reparsed.castling_rights,
            CASTLE_DARK_KINGSIDE | CASTLE_DARK_QUEENSIDE
        );
        assert_eq!(reparsed.en_passant_square, None);
        assert_eq!(reparsed.halfmove_clock, 4);
        assert_eq!(reparsed.fullmove_number, 6);

        let light_castle = CASTLE_LIGHT_KINGSIDE | CASTLE_LIGHT_QUEENSIDE;
        assert_eq!(reparsed.castling_rights & light_castle, 0);
    }
}
