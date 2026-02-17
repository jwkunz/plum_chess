use crate::game_state::{chess_types::*, game_state::GameState};
use crate::moves::move_descriptions::*;
use crate::utils::algebraic::{algebraic_to_square, square_to_algebraic};

pub fn move_description_to_long_algebraic(
    move_description: u64,
    game_state: &GameState,
) -> Result<String, String> {
    let from = move_from(move_description);
    let to = move_to(move_description);
    let promotion_code = move_promotion_piece_code(move_description);
    let moved_piece_code = move_moved_piece_code(move_description);

    let (color_on_from, piece_on_from) = piece_on_square(game_state, from)
        .ok_or_else(|| format!("No piece found on from-square {}", from))?;
    let moved_piece = piece_kind_from_code(moved_piece_code)
        .ok_or_else(|| format!("Invalid moved-piece code in move description: {moved_piece_code}"))?;

    if moved_piece != piece_on_from {
        return Err(format!(
            "Move description moved-piece mismatch: encoded={moved_piece:?}, board={piece_on_from:?}"
        ));
    }

    if color_on_from != game_state.side_to_move {
        return Err("From-square piece does not belong to side to move".to_owned());
    }

    let mut out = String::new();
    out.push_str(&square_to_algebraic(from)?);
    out.push_str(&square_to_algebraic(to)?);

    if promotion_code != NO_PIECE_CODE {
        let promotion_piece = piece_kind_from_code(promotion_code)
            .ok_or_else(|| format!("Invalid promotion piece code: {promotion_code}"))?;
        out.push(promotion_to_char(promotion_piece)?);
    }

    Ok(out)
}

pub fn long_algebraic_to_move_description(
    long_algebraic: &str,
    game_state: &GameState,
) -> Result<u64, String> {
    let bytes = long_algebraic.as_bytes();
    if bytes.len() != 4 && bytes.len() != 5 {
        return Err(format!("Invalid long algebraic move: {long_algebraic}"));
    }

    let from = algebraic_to_square(&long_algebraic[0..2])?;
    let to = algebraic_to_square(&long_algebraic[2..4])?;

    let (moving_color, moved_piece) = piece_on_square(game_state, from)
        .ok_or_else(|| format!("No piece on from-square: {}", &long_algebraic[0..2]))?;

    if moving_color != game_state.side_to_move {
        return Err("Attempted to move a piece that is not on side to move".to_owned());
    }

    let target_piece = piece_on_square(game_state, to);
    let mut captured_piece = target_piece.map(|(_, piece)| piece);
    let mut flags = 0u64;

    if captured_piece.is_some() {
        flags |= FLAG_CAPTURE;
    }

    if moved_piece == PieceKind::Pawn && from.abs_diff(to) == 16 {
        flags |= FLAG_DOUBLE_PAWN_PUSH;
    }

    if moved_piece == PieceKind::King && from.abs_diff(to) == 2 {
        flags |= FLAG_CASTLING;
    }

    if moved_piece == PieceKind::Pawn
        && game_state.en_passant_square == Some(to)
        && (from % 8 != to % 8)
        && target_piece.is_none()
    {
        let capture_square = if moving_color == Color::Light {
            to.checked_sub(8)
                .ok_or("Invalid en-passant capture square (light)")?
        } else {
            to.checked_add(8)
                .ok_or("Invalid en-passant capture square (dark)")?
        };

        let capture_piece = piece_on_square(game_state, capture_square);
        match capture_piece {
            Some((color, PieceKind::Pawn)) if color != moving_color => {
                captured_piece = Some(PieceKind::Pawn);
                flags |= FLAG_CAPTURE | FLAG_EN_PASSANT;
            }
            _ => {
                return Err("En-passant target set but no capturable pawn found".to_owned());
            }
        }
    }

    let promotion_piece = if bytes.len() == 5 {
        if moved_piece != PieceKind::Pawn {
            return Err("Only pawns may promote".to_owned());
        }

        let rank = to / 8;
        if rank != 0 && rank != 7 {
            return Err("Promotion move must end on back rank".to_owned());
        }

        Some(char_to_promotion(bytes[4] as char)?)
    } else {
        if moved_piece == PieceKind::Pawn {
            let rank = to / 8;
            if rank == 0 || rank == 7 {
                return Err("Missing promotion piece in long algebraic move".to_owned());
            }
        }
        None
    };

    Ok(pack_move_description(
        from,
        to,
        moved_piece,
        captured_piece,
        promotion_piece,
        flags,
    ))
}

fn promotion_to_char(piece_kind: PieceKind) -> Result<char, String> {
    match piece_kind {
        PieceKind::Knight => Ok('n'),
        PieceKind::Bishop => Ok('b'),
        PieceKind::Rook => Ok('r'),
        PieceKind::Queen => Ok('q'),
        _ => Err(format!("Invalid promotion piece: {piece_kind:?}")),
    }
}

fn char_to_promotion(ch: char) -> Result<PieceKind, String> {
    match ch.to_ascii_lowercase() {
        'n' => Ok(PieceKind::Knight),
        'b' => Ok(PieceKind::Bishop),
        'r' => Ok(PieceKind::Rook),
        'q' => Ok(PieceKind::Queen),
        _ => Err(format!("Invalid promotion piece character: {ch}")),
    }
}

fn piece_on_square(game_state: &GameState, square: Square) -> Option<(Color, PieceKind)> {
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
                return Some((color, piece));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{long_algebraic_to_move_description, move_description_to_long_algebraic};
    use crate::moves::move_descriptions::{
        FLAG_CASTLING, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT,
    };
    use crate::utils::fen_parser::parse_fen;

    #[test]
    fn long_algebraic_round_trip_simple_move() {
        let game_state = parse_fen("8/8/8/8/8/8/4P3/4K3 w - - 0 1").expect("FEN should parse");
        let move_description =
            long_algebraic_to_move_description("e2e4", &game_state).expect("move should parse");

        let round_trip = move_description_to_long_algebraic(move_description, &game_state)
            .expect("move description should convert");
        assert_eq!(round_trip, "e2e4");
        assert_ne!(move_description & FLAG_DOUBLE_PAWN_PUSH, 0);
    }

    #[test]
    fn long_algebraic_round_trip_promotion() {
        let game_state = parse_fen("8/P7/8/8/8/8/8/k6K w - - 0 1").expect("FEN should parse");
        let move_description =
            long_algebraic_to_move_description("a7a8q", &game_state).expect("move should parse");
        let round_trip = move_description_to_long_algebraic(move_description, &game_state)
            .expect("move description should convert");

        assert_eq!(round_trip, "a7a8q");
    }

    #[test]
    fn long_algebraic_detects_castling_and_en_passant() {
        let castle_state =
            parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").expect("FEN should parse");
        let castle_move =
            long_algebraic_to_move_description("e1g1", &castle_state).expect("castle should parse");
        assert_ne!(castle_move & FLAG_CASTLING, 0);

        let en_passant_state =
            parse_fen("8/8/8/3pP3/8/8/8/8 w - d6 0 1").expect("FEN should parse");
        let ep_move = long_algebraic_to_move_description("e5d6", &en_passant_state)
            .expect("en-passant should parse");
        assert_ne!(ep_move & FLAG_EN_PASSANT, 0);
    }
}
