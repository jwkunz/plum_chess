//! UCI protocol front-end and command loop.
//!
//! Parses UCI commands, maintains current position state, routes `go` requests
//! to the selected engine implementation, and emits protocol-compliant output.

use std::io::{self, BufRead, Write};

use crate::engines::engine_greedy::GreedyEngine;
use crate::engines::engine_iterative_v13::IterativeEngine;
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
    threads: usize,
    own_book: bool,
    ponder: bool,
    analyse_mode: bool,
    chess960: bool,
    debug_mode: bool,
}

impl UciState {
    fn new() -> Self {
        let skill_level = 1;
        let hash_mb = 64usize;
        let threads = 1usize;
        let own_book = true;
        let mut engine = build_engine(skill_level);
        let _ = engine.set_option("Hash", &hash_mb.to_string());
        let _ = engine.set_option("Threads", &threads.to_string());
        let _ = engine.set_option("OwnBook", if own_book { "true" } else { "false" });
        Self {
            game_state: GameState::new_game(),
            engine,
            skill_level,
            fixed_depth_override: None,
            hash_mb,
            threads,
            own_book,
            ponder: false,
            analyse_mode: false,
            chess960: false,
            debug_mode: false,
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
                writeln!(out, "option name Threads type spin default 1 min 1 max 128")?;
                writeln!(out, "option name Ponder type check default false")?;
                writeln!(out, "option name UCI_AnalyseMode type check default false")?;
                writeln!(out, "option name UCI_Chess960 type check default false")?;
                writeln!(out, "option name OwnBook type check default true")?;
                writeln!(
                    out,
                    "option name TimeStrategy type combo default adaptive var adaptive var fraction20"
                )?;
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
            "ponderhit" => {
                // Search is synchronous; no ponder thread to wake up.
            }
            "debug" => {
                let mode = parts.next().unwrap_or_default();
                self.debug_mode = mode.eq_ignore_ascii_case("on");
            }
            "register" => {
                // Registration is not required by this engine.
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
            let _ = self.engine.set_option("Threads", &self.threads.to_string());
            let _ = self
                .engine
                .set_option("Ponder", if self.ponder { "true" } else { "false" });
            let _ = self.engine.set_option(
                "UCI_AnalyseMode",
                if self.analyse_mode { "true" } else { "false" },
            );
            let _ = self
                .engine
                .set_option("UCI_Chess960", if self.chess960 { "true" } else { "false" });
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
        } else if name.eq_ignore_ascii_case("Threads") {
            let parsed = value
                .parse::<usize>()
                .map_err(|_| format!("invalid Threads value '{}'", value))?;
            self.threads = parsed.max(1);
            self.engine
                .set_option("Threads", &self.threads.to_string())?;
        } else if name.eq_ignore_ascii_case("Ponder") {
            let lower = value.to_ascii_lowercase();
            self.ponder = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.engine
                .set_option("Ponder", if self.ponder { "true" } else { "false" })?;
        } else if name.eq_ignore_ascii_case("UCI_AnalyseMode") {
            let lower = value.to_ascii_lowercase();
            self.analyse_mode = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.engine.set_option(
                "UCI_AnalyseMode",
                if self.analyse_mode { "true" } else { "false" },
            )?;
        } else if name.eq_ignore_ascii_case("UCI_Chess960") {
            let lower = value.to_ascii_lowercase();
            self.chess960 = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.engine
                .set_option("UCI_Chess960", if self.chess960 { "true" } else { "false" })?;
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
        let mut params = parse_go_params(line, &self.game_state)?;
        if params.depth.is_none() {
            params.depth = self.fixed_depth_override;
        }
        let result = self.engine.choose_move(&self.game_state, &params)?;

        for info in &result.info_lines {
            writeln!(out, "{}", info).map_err(|e| e.to_string())?;
        }

        if let Some(best_move) = result.best_move {
            let lan = move_description_to_long_algebraic(best_move, &self.game_state)?;
            if let Some(ponder_move) = result.ponder_move {
                let next_state = apply_move(&self.game_state, best_move)?;
                if let Ok(ponder_lan) = move_description_to_long_algebraic(ponder_move, &next_state)
                {
                    writeln!(out, "bestmove {} ponder {}", lan, ponder_lan)
                        .map_err(|e| e.to_string())?;
                } else {
                    writeln!(out, "bestmove {}", lan).map_err(|e| e.to_string())?;
                }
            } else {
                writeln!(out, "bestmove {}", lan).map_err(|e| e.to_string())?;
            }
        } else {
            writeln!(out, "bestmove 0000").map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

fn parse_go_params(line: &str, game_state: &GameState) -> Result<GoParams, String> {
    let mut params = GoParams::default();
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                i += 1;
                params.depth = tokens.get(i).and_then(|x| x.parse::<u8>().ok());
            }
            "nodes" => {
                i += 1;
                params.nodes = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "mate" => {
                i += 1;
                params.mate = tokens.get(i).and_then(|x| x.parse::<u8>().ok());
            }
            "movetime" => {
                i += 1;
                params.movetime_ms = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "ponder" => {
                params.ponder = true;
            }
            "infinite" => {
                params.infinite = true;
            }
            "wtime" => {
                i += 1;
                params.wtime_ms = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "btime" => {
                i += 1;
                params.btime_ms = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "winc" => {
                i += 1;
                params.winc_ms = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "binc" => {
                i += 1;
                params.binc_ms = tokens.get(i).and_then(|x| x.parse::<u64>().ok());
            }
            "movestogo" => {
                i += 1;
                params.movestogo = tokens.get(i).and_then(|x| x.parse::<u16>().ok());
            }
            "searchmoves" => {
                i += 1;
                let mut moves = Vec::<u64>::new();
                while i < tokens.len() && !is_go_keyword(tokens[i]) {
                    let mv = long_algebraic_to_move_description(tokens[i], game_state)?;
                    moves.push(mv);
                    i += 1;
                }
                i = i.saturating_sub(1);
                params.searchmoves = Some(moves);
            }
            _ => {}
        }
        i += 1;
    }
    Ok(params)
}

fn is_go_keyword(token: &str) -> bool {
    matches!(
        token,
        "go" | "depth"
            | "movetime"
            | "wtime"
            | "btime"
            | "winc"
            | "binc"
            | "movestogo"
            | "searchmoves"
            | "nodes"
            | "mate"
            | "ponder"
            | "infinite"
    )
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
    fn setoption_chess960_parse() {
        let mut state = UciState::new();
        assert!(!state.chess960);
        state
            .handle_setoption("setoption name UCI_Chess960 value true")
            .expect("chess960 should parse");
        assert!(state.chess960);
    }

    #[test]
    fn parse_go_params_keeps_clock_fields_without_forcing_movetime() {
        let game_state = crate::game_state::game_state::GameState::new_game();
        let params = super::parse_go_params(
            "go wtime 120000 btime 60000 winc 1000 binc 1000",
            &game_state,
        )
        .expect("go params should parse");
        assert_eq!(params.movetime_ms, None);
        assert_eq!(params.wtime_ms, Some(120_000));
        assert_eq!(params.btime_ms, Some(60_000));
        assert_eq!(params.winc_ms, Some(1_000));
        assert_eq!(params.binc_ms, Some(1_000));
    }

    #[test]
    fn parse_go_params_parses_movestogo_and_searchmoves() {
        let game_state = crate::game_state::game_state::GameState::new_game();
        let params =
            super::parse_go_params("go movestogo 24 searchmoves e2e4 d2d4 depth 6", &game_state)
                .expect("go params should parse");
        assert_eq!(params.movestogo, Some(24));
        assert_eq!(params.depth, Some(6));
        let moves = params.searchmoves.expect("searchmoves should parse");
        assert_eq!(moves.len(), 2);
    }

    #[test]
    fn parse_go_params_parses_nodes_mate_and_modes() {
        let game_state = crate::game_state::game_state::GameState::new_game();
        let params = super::parse_go_params("go nodes 50000 mate 3 ponder infinite", &game_state)
            .expect("go params should parse");
        assert_eq!(params.nodes, Some(50_000));
        assert_eq!(params.mate, Some(3));
        assert!(params.ponder);
        assert!(params.infinite);
    }
}
