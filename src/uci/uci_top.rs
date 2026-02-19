//! UCI protocol front-end and command loop.
//!
//! Parses UCI commands, maintains current position state, routes `go` requests
//! to the selected engine implementation, and emits protocol-compliant output.

use std::io::{self, BufRead, Write};

use crate::engines::engine_greedy::GreedyEngine;
use crate::engines::engine_iterative_v7::IterativeEngine;
use crate::engines::engine_random::RandomEngine;
use crate::engines::engine_trait::{Engine, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::utils::long_algebraic::{
    long_algebraic_to_move_description, move_description_to_long_algebraic,
};

const UCI_ENGINE_NAME: &str = "Plum Chess";
const UCI_ENGINE_AUTHOR: &str = "jwkunz using Codex";

pub fn run_stdio_loop() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut uci = UciState::new();

    for line in stdin.lock().lines() {
        let line = line?;
        let should_quit = uci.handle_command(&line, &mut stdout)?;
        stdout.flush()?;
        if should_quit {
            break;
        }
    }

    Ok(())
}

struct UciState {
    game_state: GameState,
    engine: Box<dyn Engine>,
    skill_level: u8,
    fixed_depth_override: Option<u8>,
    hash_mb: usize,
    own_book: bool,
}

impl UciState {
    fn new() -> Self {
        let skill_level = 1;
        let hash_mb = 64usize;
        let own_book = true;
        let mut engine = build_engine(skill_level);
        let _ = engine.set_option("Hash", &hash_mb.to_string());
        let _ = engine.set_option("OwnBook", if own_book { "true" } else { "false" });
        Self {
            game_state: GameState::new_game(),
            engine,
            skill_level,
            fixed_depth_override: None,
            hash_mb,
            own_book,
        }
    }

    fn handle_command(&mut self, line: &str, out: &mut impl Write) -> io::Result<bool> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or_default();

        match cmd {
            "uci" => {
                writeln!(out, "id name {}", UCI_ENGINE_NAME)?;
                writeln!(out, "id author {}", UCI_ENGINE_AUTHOR)?;
                writeln!(
                    out,
                    "option name Skill Level type spin default 1 min 1 max 10"
                )?;
                writeln!(
                    out,
                    "option name FixedDepth type spin default 0 min 0 max 64"
                )?;
                writeln!(out, "option name Hash type spin default 64 min 1 max 4096")?;
                writeln!(out, "option name OwnBook type check default true")?;
                writeln!(out, "uciok")?;
            }
            "isready" => {
                writeln!(out, "readyok")?;
            }
            "setoption" => {
                if let Err(err) = self.handle_setoption(trimmed) {
                    writeln!(out, "info string setoption error: {}", err)?;
                }
            }
            "ucinewgame" => {
                self.game_state = GameState::new_game();
                self.engine.new_game();
            }
            "position" => {
                if let Err(err) = self.handle_position(trimmed) {
                    writeln!(out, "info string position error: {}", err)?;
                }
            }
            "go" => {
                if let Err(err) = self.handle_go(trimmed, out) {
                    writeln!(out, "info string go error: {}", err)?;
                    writeln!(out, "bestmove 0000")?;
                }
            }
            "stop" => {
                // Search is currently synchronous; no-op for now.
            }
            "quit" => {
                return Ok(true);
            }
            _ => {
                // Unknown commands are ignored for UCI compatibility.
            }
        }

        Ok(false)
    }

    fn handle_setoption(&mut self, line: &str) -> Result<(), String> {
        let mut tokens = line.split_whitespace();
        let _ = tokens.next(); // setoption

        let mut name_tokens = Vec::<String>::new();
        let mut value_tokens = Vec::<String>::new();
        let mut mode = "";

        while let Some(tok) = tokens.next() {
            match tok {
                "name" => mode = "name",
                "value" => mode = "value",
                _ if mode == "name" => name_tokens.push(tok.to_owned()),
                _ if mode == "value" => value_tokens.push(tok.to_owned()),
                _ => {}
            }
        }

        let name = name_tokens.join(" ");
        let value = value_tokens.join(" ");

        if name.eq_ignore_ascii_case("Skill Level") {
            let parsed = value
                .parse::<u8>()
                .map_err(|_| format!("invalid Skill Level value '{}'", value))?;
            self.skill_level = parsed;
            self.engine = build_engine(self.skill_level);
            let _ = self.engine.set_option("Hash", &self.hash_mb.to_string());
            let _ = self
                .engine
                .set_option("OwnBook", if self.own_book { "true" } else { "false" });
            self.engine.new_game();
        } else if name.eq_ignore_ascii_case("FixedDepth") {
            let parsed = value
                .parse::<u8>()
                .map_err(|_| format!("invalid FixedDepth value '{}'", value))?;
            self.fixed_depth_override = if parsed == 0 { None } else { Some(parsed) };
        } else if name.eq_ignore_ascii_case("Hash") {
            let parsed = value
                .parse::<usize>()
                .map_err(|_| format!("invalid Hash value '{}'", value))?;
            self.hash_mb = parsed.max(1);
            self.engine.set_option("Hash", &self.hash_mb.to_string())?;
        } else if name.eq_ignore_ascii_case("OwnBook") {
            let lower = value.to_ascii_lowercase();
            self.own_book = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.engine
                .set_option("OwnBook", if self.own_book { "true" } else { "false" })?;
        } else {
            self.engine.set_option(&name, &value)?;
        }

        Ok(())
    }

    fn handle_position(&mut self, line: &str) -> Result<(), String> {
        let mut tokens = line.split_whitespace().peekable();
        let _ = tokens.next(); // "position"

        let mut base_state = if let Some(tok) = tokens.next() {
            match tok {
                "startpos" => GameState::new_game(),
                "fen" => {
                    let mut fen_parts = Vec::<String>::new();
                    while let Some(next) = tokens.peek() {
                        if *next == "moves" {
                            break;
                        }
                        fen_parts.push(tokens.next().unwrap_or_default().to_owned());
                    }
                    if fen_parts.is_empty() {
                        return Err("missing FEN after 'position fen'".to_owned());
                    }
                    let fen = fen_parts.join(" ");
                    GameState::from_fen(&fen)?
                }
                other => return Err(format!("unsupported position token '{}'", other)),
            }
        } else {
            return Err("incomplete position command".to_owned());
        };

        if tokens.peek().copied() == Some("moves") {
            let _ = tokens.next();
            for lan in tokens {
                let mv = long_algebraic_to_move_description(lan, &base_state)?;
                base_state = apply_move(&base_state, mv)?;
            }
        }

        self.game_state = base_state;
        Ok(())
    }

    fn handle_go(&mut self, line: &str, out: &mut impl Write) -> Result<(), String> {
        let mut params = parse_go_params(line);
        if params.depth.is_none() {
            params.depth = self.fixed_depth_override;
        }
        apply_basic_time_management(&self.game_state, &mut params);
        let result = self.engine.choose_move(&self.game_state, &params)?;

        for info in &result.info_lines {
            writeln!(out, "{}", info).map_err(|e| e.to_string())?;
        }

        if let Some(best_move) = result.best_move {
            let lan = move_description_to_long_algebraic(best_move, &self.game_state)?;
            writeln!(out, "bestmove {}", lan).map_err(|e| e.to_string())?;
        } else {
            writeln!(out, "bestmove 0000").map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

fn apply_basic_time_management(game_state: &GameState, params: &mut GoParams) {
    // Keep existing explicit limits unchanged.
    if params.movetime_ms.is_some() {
        return;
    }

    let remaining_ms = match game_state.side_to_move {
        crate::game_state::chess_types::Color::Light => params.wtime_ms,
        crate::game_state::chess_types::Color::Dark => params.btime_ms,
    };

    if let Some(clock) = remaining_ms {
        // Spend roughly one twentieth of remaining clock to better sustain longer games.
        params.movetime_ms = Some((clock / 20).max(1));
    }
}

fn parse_go_params(line: &str) -> GoParams {
    let mut params = GoParams::default();
    let mut tokens = line.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        match tok {
            "depth" => params.depth = tokens.next().and_then(|x| x.parse::<u8>().ok()),
            "movetime" => params.movetime_ms = tokens.next().and_then(|x| x.parse::<u64>().ok()),
            "wtime" => params.wtime_ms = tokens.next().and_then(|x| x.parse::<u64>().ok()),
            "btime" => params.btime_ms = tokens.next().and_then(|x| x.parse::<u64>().ok()),
            "winc" => params.winc_ms = tokens.next().and_then(|x| x.parse::<u64>().ok()),
            "binc" => params.binc_ms = tokens.next().and_then(|x| x.parse::<u64>().ok()),
            _ => {}
        }
    }
    params
}

fn build_engine(skill_level: u8) -> Box<dyn Engine> {
    match skill_level {
        1 => Box::new(RandomEngine::new()),
        2 => Box::new(GreedyEngine::new()),
        3 => Box::new(IterativeEngine::new_standard(2)),
        4 => Box::new(IterativeEngine::new_alpha_zero(2)),
        5 => Box::new(IterativeEngine::new_standard(3)),
        6 => Box::new(IterativeEngine::new_alpha_zero(3)),
        7 => Box::new(IterativeEngine::new_standard(4)),
        8 => Box::new(IterativeEngine::new_alpha_zero(4)),
        9 => Box::new(IterativeEngine::new_standard(5)),
        10 => Box::new(IterativeEngine::new_alpha_zero(5)),
        _ => Box::new(IterativeEngine::new_alpha_zero(10)),
    }
}

#[cfg(test)]
mod tests {
    use super::UciState;
    use crate::engines::engine_trait::GoParams;
    use crate::game_state::chess_types::Color;
    use crate::game_state::game_state::GameState;

    #[test]
    fn position_startpos_with_moves_updates_state() {
        let mut state = UciState::new();
        state
            .handle_position("position startpos moves e2e4 e7e5 g1f3")
            .expect("position command should parse");

        assert_eq!(
            state.game_state.side_to_move,
            crate::game_state::chess_types::Color::Dark
        );
    }

    #[test]
    fn position_fen_without_moves_updates_state() {
        let mut state = UciState::new();
        state
            .handle_position("position fen 8/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .expect("position fen should parse");

        assert_eq!(state.game_state.get_fen(), "8/8/8/8/8/8/4P3/4K3 w - - 0 1");
    }

    #[test]
    fn setoption_skill_level_switches_engine() {
        let mut state = UciState::new();
        assert_eq!(state.skill_level, 1);

        state
            .handle_setoption("setoption name Skill Level value 2")
            .expect("setoption should parse");
        assert_eq!(state.skill_level, 2);

        state
            .handle_setoption("setoption name Skill Level value 3")
            .expect("setoption should parse");
        assert_eq!(state.skill_level, 3);
    }

    #[test]
    fn setoption_skill_level_allows_out_of_range_and_uses_fallback_engine_mapping() {
        let mut state = UciState::new();
        state
            .handle_setoption("setoption name Skill Level value 42")
            .expect("setoption should parse");
        assert_eq!(state.skill_level, 42);
    }

    #[test]
    fn setoption_fixed_depth_sets_override() {
        let mut state = UciState::new();
        assert_eq!(state.fixed_depth_override, None);

        state
            .handle_setoption("setoption name FixedDepth value 4")
            .expect("setoption should parse");
        assert_eq!(state.fixed_depth_override, Some(4));

        state
            .handle_setoption("setoption name FixedDepth value 0")
            .expect("setoption should parse");
        assert_eq!(state.fixed_depth_override, None);
    }

    #[test]
    fn setoption_hash_and_ownbook_parse() {
        let mut state = UciState::new();
        state
            .handle_setoption("setoption name Hash value 128")
            .expect("hash should parse");
        assert_eq!(state.hash_mb, 128);

        state
            .handle_setoption("setoption name OwnBook value false")
            .expect("ownbook should parse");
        assert!(!state.own_book);
    }

    #[test]
    fn go_time_management_uses_twentieth_of_remaining_time_for_side_to_move() {
        let game = GameState::new_game();
        assert_eq!(game.side_to_move, Color::Light);
        let mut params = GoParams {
            wtime_ms: Some(120_000),
            btime_ms: Some(60_000),
            ..GoParams::default()
        };
        super::apply_basic_time_management(&game, &mut params);
        assert_eq!(params.movetime_ms, Some(6_000));

        let game_dark =
            GameState::from_fen("8/8/8/8/8/8/8/4k2K b - - 0 1").expect("fen should parse");
        let mut params_dark = GoParams {
            wtime_ms: Some(120_000),
            btime_ms: Some(60_000),
            ..GoParams::default()
        };
        super::apply_basic_time_management(&game_dark, &mut params_dark);
        assert_eq!(params_dark.movetime_ms, Some(3_000));
    }

    #[test]
    fn go_time_management_does_not_override_explicit_movetime() {
        let game = GameState::new_game();
        let mut params = GoParams {
            movetime_ms: Some(250),
            wtime_ms: Some(120_000),
            btime_ms: Some(60_000),
            ..GoParams::default()
        };
        super::apply_basic_time_management(&game, &mut params);
        assert_eq!(params.movetime_ms, Some(250));
    }
}
