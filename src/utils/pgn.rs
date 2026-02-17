//! PGN read/write utilities for game history interchange.
//!
//! Serializes move history and headers to PGN text and parses PGN back into
//! engine state sequences suitable for replay and analysis workflows.

use std::collections::BTreeMap;

use crate::game_state::chess_rules::STARTING_POSITION_FEN;
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::utils::long_algebraic::{
    long_algebraic_to_move_description, move_description_to_long_algebraic,
};

#[derive(Debug, Clone)]
pub struct PgnGame {
    pub headers: BTreeMap<String, String>,
    pub initial_state: GameState,
    pub move_history: Vec<u64>,
    pub final_state: GameState,
    pub result: String,
}

pub fn write_pgn(
    initial_state: &GameState,
    move_history: &[u64],
    result: &str,
) -> Result<String, String> {
    let mut headers = BTreeMap::<String, String>::new();
    headers.insert("Event".to_owned(), "Plum Chess Game".to_owned());
    headers.insert("Site".to_owned(), "Local".to_owned());
    headers.insert("Date".to_owned(), "????.??.??".to_owned());
    headers.insert("Round".to_owned(), "-".to_owned());
    headers.insert("White".to_owned(), "White".to_owned());
    headers.insert("Black".to_owned(), "Black".to_owned());
    headers.insert("Result".to_owned(), normalize_result(result).to_owned());

    let initial_fen = initial_state.get_fen();
    if initial_fen != STARTING_POSITION_FEN {
        headers.insert("SetUp".to_owned(), "1".to_owned());
        headers.insert("FEN".to_owned(), initial_fen);
    }

    write_pgn_with_headers(initial_state, move_history, &headers)
}

pub fn write_pgn_with_headers(
    initial_state: &GameState,
    move_history: &[u64],
    headers: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut out = String::new();

    for (key, value) in headers {
        out.push_str(&format!("[{} \"{}\"]\n", key, escape_pgn_value(value)));
    }
    out.push('\n');

    let mut state = initial_state.clone();
    let mut movetext_parts = Vec::<String>::with_capacity(move_history.len() + 1);
    for (ply, mv) in move_history.iter().enumerate() {
        let lan = move_description_to_long_algebraic(*mv, &state)?;
        if ply % 2 == 0 {
            movetext_parts.push(format!("{}. {}", (ply / 2) + 1, lan));
        } else {
            movetext_parts.push(lan);
        }
        state = apply_move(&state, *mv)?;
    }

    let result = headers
        .get("Result")
        .map(|x| normalize_result(x))
        .unwrap_or("*");
    movetext_parts.push(result.to_owned());
    out.push_str(&movetext_parts.join(" "));
    out.push('\n');

    Ok(out)
}

pub fn read_pgn(pgn: &str) -> Result<PgnGame, String> {
    let mut headers = BTreeMap::<String, String>::new();
    let mut movetext_lines = Vec::<String>::new();

    for line in pgn.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('[') {
            let (k, v) = parse_header_line(trimmed)?;
            headers.insert(k, v);
        } else {
            movetext_lines.push(trimmed.to_owned());
        }
    }

    let initial_state = if headers.get("SetUp").map(|x| x.as_str()) == Some("1") {
        let fen = headers
            .get("FEN")
            .ok_or("PGN SetUp=1 is present but FEN header is missing")?;
        GameState::from_fen(fen)?
    } else {
        GameState::new_game()
    };

    let mut state = initial_state.clone();
    let mut move_history = Vec::<u64>::new();
    let mut result = "*".to_owned();

    let movetext = strip_pgn_comments_and_variations(&movetext_lines.join(" "));
    for token in movetext.split_whitespace() {
        if is_move_number_token(token) {
            continue;
        }

        let cleaned = trim_annotation_suffix(token);
        if is_result_token(cleaned) {
            result = normalize_result(cleaned).to_owned();
            break;
        }

        let mv = long_algebraic_to_move_description(cleaned, &state)?;
        state = apply_move(&state, mv)?;
        move_history.push(mv);
    }

    if let Some(header_result) = headers.get("Result") {
        result = normalize_result(header_result).to_owned();
    }

    Ok(PgnGame {
        headers,
        initial_state,
        move_history,
        final_state: state,
        result,
    })
}

fn parse_header_line(line: &str) -> Result<(String, String), String> {
    if !line.starts_with('[') || !line.ends_with(']') {
        return Err(format!("Invalid PGN header line: {line}"));
    }
    let inner = &line[1..line.len() - 1];
    let mut parts = inner.splitn(2, ' ');
    let key = parts
        .next()
        .ok_or_else(|| format!("Invalid PGN header key: {line}"))?
        .trim();
    let value_raw = parts
        .next()
        .ok_or_else(|| format!("Invalid PGN header value: {line}"))?
        .trim();

    if !value_raw.starts_with('"') || !value_raw.ends_with('"') || value_raw.len() < 2 {
        return Err(format!("Invalid quoted PGN header value: {line}"));
    }
    let value = value_raw[1..value_raw.len() - 1].replace("\\\"", "\"");
    Ok((key.to_owned(), value))
}

fn strip_pgn_comments_and_variations(text: &str) -> String {
    let mut out = String::new();
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;

    for ch in text.chars() {
        match ch {
            '{' => brace_depth = brace_depth.saturating_add(1),
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ if brace_depth == 0 && paren_depth == 0 => out.push(ch),
            _ => {}
        }
    }

    out
}

fn is_move_number_token(token: &str) -> bool {
    if token.ends_with('.') {
        return token
            .trim_end_matches('.')
            .chars()
            .all(|c| c.is_ascii_digit());
    }
    if token.contains("...") {
        let head = token.split("...").next().unwrap_or_default();
        return !head.is_empty() && head.chars().all(|c| c.is_ascii_digit());
    }
    false
}

fn trim_annotation_suffix(token: &str) -> &str {
    token.trim_end_matches(|c: char| matches!(c, '+' | '#' | '!' | '?'))
}

fn is_result_token(token: &str) -> bool {
    matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*")
}

fn normalize_result(result: &str) -> &str {
    if is_result_token(result) {
        result
    } else {
        "*"
    }
}

fn escape_pgn_value(value: &str) -> String {
    value.replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::{read_pgn, write_pgn, write_pgn_with_headers};
    use std::collections::BTreeMap;

    use crate::game_state::game_state::GameState;
    use crate::move_generation::legal_move_apply::apply_move;
    use crate::utils::long_algebraic::long_algebraic_to_move_description;

    #[test]
    fn pgn_round_trip_start_position_history() {
        let mut game = GameState::new_game();
        let mut history = Vec::<u64>::new();

        for lan in ["e2e4", "e7e5", "g1f3", "b8c6"] {
            let mv = long_algebraic_to_move_description(lan, &game).expect("LAN should parse");
            game = apply_move(&game, mv).expect("move should apply");
            history.push(mv);
        }

        let pgn = write_pgn(&GameState::new_game(), &history, "*").expect("PGN should write");
        let parsed = read_pgn(&pgn).expect("PGN should parse");

        assert_eq!(parsed.move_history, history);
        assert_eq!(parsed.final_state.get_fen(), game.get_fen());
        assert_eq!(parsed.result, "*");
    }

    #[test]
    fn pgn_round_trip_custom_fen_setup() {
        let initial =
            GameState::from_fen("8/8/8/8/8/8/4P3/4K3 w - - 0 1").expect("FEN should parse");
        let mv = long_algebraic_to_move_description("e2e4", &initial).expect("LAN should parse");
        let history = vec![mv];

        let mut headers = BTreeMap::<String, String>::new();
        headers.insert("Event".to_owned(), "Custom".to_owned());
        headers.insert("Result".to_owned(), "1-0".to_owned());
        headers.insert("SetUp".to_owned(), "1".to_owned());
        headers.insert("FEN".to_owned(), initial.get_fen());

        let pgn = write_pgn_with_headers(&initial, &history, &headers).expect("PGN should write");
        dbg!(&pgn);
        let parsed = read_pgn(&pgn).expect("PGN should parse");

        assert_eq!(parsed.initial_state.get_fen(), initial.get_fen());
        assert_eq!(parsed.move_history, history);
        assert_eq!(parsed.result, "1-0");
    }
}
