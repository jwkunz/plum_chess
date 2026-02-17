use std::io::{self, BufRead, Write};

use crate::engines::engine_random::RandomEngine;
use crate::engines::engine_trait::{Engine, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::utils::long_algebraic::{
    long_algebraic_to_move_description, move_description_to_long_algebraic,
};

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
}

impl UciState {
    fn new() -> Self {
        Self {
            game_state: GameState::new_game(),
            engine: Box::new(RandomEngine::new()),
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
                writeln!(out, "id name {}", self.engine.name())?;
                writeln!(out, "id author {}", self.engine.author())?;
                writeln!(out, "uciok")?;
            }
            "isready" => {
                writeln!(out, "readyok")?;
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
        let params = parse_go_params(line);
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

#[cfg(test)]
mod tests {
    use super::UciState;

    #[test]
    fn position_startpos_with_moves_updates_state() {
        let mut state = UciState::new();
        state
            .handle_position("position startpos moves e2e4 e7e5 g1f3")
            .expect("position command should parse");

        assert_eq!(state.game_state.side_to_move, crate::game_state::chess_types::Color::Dark);
    }

    #[test]
    fn position_fen_without_moves_updates_state() {
        let mut state = UciState::new();
        state
            .handle_position("position fen 8/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .expect("position fen should parse");

        assert_eq!(state.game_state.get_fen(), "8/8/8/8/8/8/4P3/4K3 w - - 0 1");
    }
}
