//! FEN-to-GameState parser.
//!
//! Builds fully-populated incremental state from a Forsyth-Edwards Notation
//! string, including piece bitboards, rights, clocks, and occupancies.

use crate::game_state::{chess_types::*, game_state::GameState};
use crate::utils::algebraic::algebraic_to_square;

pub fn parse_fen(fen: &str) -> Result<GameState, String> {
    let mut parts = fen.split_whitespace();

    let board_part = parts.next().ok_or("Missing board layout in FEN")?;
    let side_part = parts.next().ok_or("Missing side-to-move in FEN")?;
    let castling_part = parts.next().ok_or("Missing castling rights in FEN")?;
    let en_passant_part = parts.next().ok_or("Missing en-passant square in FEN")?;
    let halfmove_part = parts.next().ok_or("Missing halfmove clock in FEN")?;
    let fullmove_part = parts.next().ok_or("Missing fullmove number in FEN")?;

    if parts.next().is_some() {
        return Err("FEN has extra trailing fields".to_owned());
    }

    let mut game_state = GameState::new_empty();

    parse_board(board_part, &mut game_state)?;
    game_state.side_to_move = parse_side_to_move(side_part)?;
    game_state.castling_rights = parse_castling_rights(castling_part)?;
    game_state.en_passant_square = parse_en_passant_square(en_passant_part)?;
    game_state.halfmove_clock = halfmove_part
        .parse::<u16>()
        .map_err(|_| format!("Invalid halfmove clock: {halfmove_part}"))?;
    game_state.fullmove_number = fullmove_part
        .parse::<u16>()
        .map_err(|_| format!("Invalid fullmove number: {fullmove_part}"))?;

    game_state.occupancy_by_color[Color::Light.index()] =
        game_state.pieces[Color::Light.index()].iter().copied().fold(0u64, |acc, bb| acc | bb);
    game_state.occupancy_by_color[Color::Dark.index()] =
        game_state.pieces[Color::Dark.index()].iter().copied().fold(0u64, |acc, bb| acc | bb);
    game_state.occupancy_all =
        game_state.occupancy_by_color[Color::Light.index()] | game_state.occupancy_by_color[Color::Dark.index()];

    Ok(game_state)
}

fn parse_board(board_part: &str, game_state: &mut GameState) -> Result<(), String> {
    let ranks: Vec<&str> = board_part.split('/').collect();
    if ranks.len() != 8 {
        return Err("Board layout must contain 8 ranks".to_owned());
    }

    for (fen_rank_idx, rank_str) in ranks.iter().enumerate() {
        let board_rank = 7usize.saturating_sub(fen_rank_idx);
        let mut file = 0usize;

        for ch in rank_str.chars() {
            if let Some(empty_count) = ch.to_digit(10) {
                let step = usize::try_from(empty_count).map_err(|_| "Digit conversion failed")?;
                if !(1..=8).contains(&step) {
                    return Err(format!("Invalid empty-square count '{ch}'"));
                }
                file += step;
                continue;
            }

            let (color, piece) = piece_from_fen_char(ch)
                .ok_or_else(|| format!("Invalid piece character '{ch}' in board layout"))?;

            if file >= 8 {
                return Err("Board rank has too many files".to_owned());
            }

            let sq = board_rank * 8 + file;
            game_state.pieces[color.index()][piece.index()] |= 1u64 << sq;
            file += 1;
        }

        if file != 8 {
            return Err("Board rank does not sum to 8 files".to_owned());
        }
    }

    Ok(())
}

fn parse_side_to_move(side_part: &str) -> Result<Color, String> {
    match side_part {
        "w" => Ok(Color::Light),
        "b" => Ok(Color::Dark),
        _ => Err(format!("Invalid side-to-move field: {side_part}")),
    }
}

fn parse_castling_rights(castling_part: &str) -> Result<CastlingRights, String> {
    if castling_part == "-" {
        return Ok(0);
    }

    let mut rights: CastlingRights = 0;

    for ch in castling_part.chars() {
        match ch {
            'K' => rights |= CASTLE_LIGHT_KINGSIDE,
            'Q' => rights |= CASTLE_LIGHT_QUEENSIDE,
            'k' => rights |= CASTLE_DARK_KINGSIDE,
            'q' => rights |= CASTLE_DARK_QUEENSIDE,
            _ => return Err(format!("Invalid castling rights character: {ch}")),
        }
    }

    Ok(rights)
}

fn parse_en_passant_square(en_passant_part: &str) -> Result<Option<Square>, String> {
    if en_passant_part == "-" {
        return Ok(None);
    }

    Ok(Some(algebraic_to_square(en_passant_part)?))
}

fn piece_from_fen_char(ch: char) -> Option<(Color, PieceKind)> {
    let color = if ch.is_ascii_uppercase() {
        Color::Light
    } else if ch.is_ascii_lowercase() {
        Color::Dark
    } else {
        return None;
    };

    let lower = ch.to_ascii_lowercase();
    let piece = match lower {
        'p' => PieceKind::Pawn,
        'n' => PieceKind::Knight,
        'b' => PieceKind::Bishop,
        'r' => PieceKind::Rook,
        'q' => PieceKind::Queen,
        'k' => PieceKind::King,
        _ => return None,
    };

    Some((color, piece))
}

#[cfg(test)]
mod tests {
    use super::parse_fen;
    use crate::game_state::chess_rules::STARTING_POSITION_FEN;
    use crate::utils::render_game_state::render_game_state;

    #[test]
    fn parse_starting_fen_and_render_board() {
        let game_state = parse_fen(STARTING_POSITION_FEN).expect("starting FEN should parse");

        println!("\n{}", render_game_state(&game_state));

        assert_eq!(game_state.side_to_move, crate::game_state::chess_types::Color::Light);
        assert_eq!(game_state.fullmove_number, 1);
        assert_eq!(game_state.halfmove_clock, 0);
    }
}
