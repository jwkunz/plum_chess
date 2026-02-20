//! UCI protocol front-end and command loop.
//!
//! Parses UCI commands, maintains current position state, routes `go` requests
//! to the selected engine implementation, and emits protocol-compliant output.

use std::io::{self, BufRead, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use crate::engines::engine_greedy::GreedyEngine;
use crate::engines::engine_iterative_v15::IterativeEngine;
use crate::engines::engine_random::RandomEngine;
use crate::engines::engine_trait::{Engine, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::utils::long_algebraic::{
    long_algebraic_to_move_description, move_description_to_long_algebraic,
};

const UCI_ENGINE_NAME: &str = "Plum Chess";
const UCI_ENGINE_AUTHOR: &str = "jwkunz using Codex";
const UCI_ENGINE_ABOUT: &str = "Plum Chess by jwkunz using Codex";

pub fn run_stdio_loop() -> io::Result<()> {
    let (output_tx, output_rx) = mpsc::channel::<String>();
    let output_thread = thread::spawn(move || -> io::Result<()> {
        for line in output_rx {
            // Acquire and release stdout lock per line so the main UCI command
            // loop can also write synchronously without deadlocking.
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            writeln!(lock, "{}", line)?;
            lock.flush()?;
        }
        Ok(())
    });

    let stdin = io::stdin();
    let mut uci = UciState::new();
    uci.set_async_info_sender(Some(output_tx.clone()));

    for line in stdin.lock().lines() {
        let line = line?;
        let mut stdout = io::stdout();
        let should_quit = uci.handle_command(&line, &mut stdout)?;
        stdout.flush()?;
        if should_quit {
            break;
        }
    }

    uci.set_async_info_sender(None);
    drop(uci);
    drop(output_tx);
    output_thread
        .join()
        .map_err(|_| io::Error::other("output thread panicked"))??;

    Ok(())
}

struct UciState {
    game_state: GameState,
    engine: Box<dyn Engine>,
    skill_level: u8,
    limit_strength: bool,
    uci_elo: u16,
    multipv: usize,
    fixed_depth_override: Option<u8>,
    hash_mb: usize,
    threads: usize,
    own_book: bool,
    ponder: bool,
    analyse_mode: bool,
    chess960: bool,
    show_wdl: bool,
    show_currline: bool,
    show_refutations: bool,
    uci_opponent: String,
    debug_mode: bool,
    time_strategy: String,
    async_search: Option<AsyncSearchHandle>,
    async_info_tx: Option<mpsc::Sender<String>>,
}

struct AsyncSearchHandle {
    stop: Arc<AtomicBool>,
    latest: Arc<Mutex<Option<crate::engines::engine_trait::EngineOutput>>>,
    error: Arc<Mutex<Option<String>>>,
    go_params: GoParams,
    is_ponder: bool,
    handle: JoinHandle<()>,
}

impl UciState {
    fn new() -> Self {
        let skill_level = 1;
        let limit_strength = false;
        let uci_elo = 1200u16;
        let multipv = 1usize;
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
            limit_strength,
            uci_elo,
            multipv,
            fixed_depth_override: None,
            hash_mb,
            threads,
            own_book,
            ponder: false,
            analyse_mode: false,
            chess960: false,
            show_wdl: false,
            show_currline: false,
            show_refutations: false,
            uci_opponent: "none none computer unknown".to_owned(),
            debug_mode: false,
            time_strategy: "adaptive".to_owned(),
            async_search: None,
            async_info_tx: None,
        }
    }

    fn set_async_info_sender(&mut self, sender: Option<mpsc::Sender<String>>) {
        self.async_info_tx = sender;
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
                    "option name UCI_LimitStrength type check default false"
                )?;
                writeln!(
                    out,
                    "option name UCI_Elo type spin default 1200 min 600 max 1800"
                )?;
                writeln!(out, "option name MultiPV type spin default 1 min 1 max 32")?;
                writeln!(
                    out,
                    "option name FixedDepth type spin default 0 min 0 max 64"
                )?;
                writeln!(out, "option name Hash type spin default 64 min 1 max 4096")?;
                writeln!(out, "option name Clear Hash type button")?;
                writeln!(out, "option name Threads type spin default 1 min 1 max 128")?;
                writeln!(out, "option name Ponder type check default false")?;
                writeln!(out, "option name UCI_AnalyseMode type check default false")?;
                writeln!(out, "option name UCI_Chess960 type check default false")?;
                writeln!(out, "option name UCI_ShowWDL type check default false")?;
                writeln!(out, "option name UCI_ShowCurrLine type check default false")?;
                writeln!(out, "option name UCI_ShowRefutations type check default false")?;
                writeln!(
                    out,
                    "option name UCI_Opponent type string default {}",
                    self.uci_opponent
                )?;
                writeln!(
                    out,
                    "option name UCI_EngineAbout type string default {}",
                    UCI_ENGINE_ABOUT
                )?;
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
                let _ = self.stop_async_search_and_collect();
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
                if let Err(err) = self.handle_stop(out) {
                    writeln!(out, "info string stop error: {}", err)?;
                    writeln!(out, "bestmove 0000")?;
                }
            }
            "ponderhit" => {
                if let Err(err) = self.handle_ponderhit(out) {
                    writeln!(out, "info string ponderhit error: {}", err)?;
                    writeln!(out, "bestmove 0000")?;
                }
            }
            "debug" => {
                let mode = parts.next().unwrap_or_default();
                self.debug_mode = mode.eq_ignore_ascii_case("on");
            }
            "register" => {
                // Registration is not required by this engine.
            }
            "quit" => {
                let _ = self.stop_async_search_and_collect();
                return Ok(true);
            }
            _ => {
                // Unknown commands are ignored for UCI compatibility.
            }
        }

        Ok(false)
    }

    fn handle_setoption(&mut self, line: &str) -> Result<(), String> {
        let _ = self.stop_async_search_and_collect();
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
            self.rebuild_engine_for_current_strength()?;
        } else if name.eq_ignore_ascii_case("UCI_LimitStrength") {
            let lower = value.to_ascii_lowercase();
            self.limit_strength = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.rebuild_engine_for_current_strength()?;
        } else if name.eq_ignore_ascii_case("UCI_Elo") {
            let parsed = value
                .parse::<u16>()
                .map_err(|_| format!("invalid UCI_Elo value '{}'", value))?;
            self.uci_elo = parsed.clamp(600, 1800);
            self.rebuild_engine_for_current_strength()?;
        } else if name.eq_ignore_ascii_case("MultiPV") {
            let parsed = value
                .parse::<usize>()
                .map_err(|_| format!("invalid MultiPV value '{}'", value))?;
            self.multipv = parsed.clamp(1, 32);
            self.engine
                .set_option("MultiPV", &self.multipv.to_string())?;
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
        } else if name.eq_ignore_ascii_case("Clear Hash") {
            // UCI button option: clear transposition state without changing position.
            self.engine.new_game();
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
        } else if name.eq_ignore_ascii_case("UCI_ShowWDL") {
            let lower = value.to_ascii_lowercase();
            self.show_wdl = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
        } else if name.eq_ignore_ascii_case("UCI_ShowCurrLine") {
            let lower = value.to_ascii_lowercase();
            self.show_currline = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
        } else if name.eq_ignore_ascii_case("UCI_ShowRefutations") {
            let lower = value.to_ascii_lowercase();
            self.show_refutations = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
            self.engine.set_option(
                "UCI_ShowRefutations",
                if self.show_refutations { "true" } else { "false" },
            )?;
        } else if name.eq_ignore_ascii_case("UCI_Opponent") {
            self.uci_opponent = value.trim().to_owned();
            self.engine.set_option("UCI_Opponent", &self.uci_opponent)?;
        } else if name.eq_ignore_ascii_case("UCI_EngineAbout") {
            // Read-only informational option in practice; accept and ignore for compatibility.
        } else if name.eq_ignore_ascii_case("TimeStrategy") {
            let normalized = value.trim().to_ascii_lowercase();
            self.time_strategy = normalized.clone();
            self.engine.set_option("TimeStrategy", &normalized)?;
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

    fn effective_skill_level(&self) -> u8 {
        if self.limit_strength {
            elo_to_skill_level(self.uci_elo)
        } else {
            self.skill_level
        }
    }

    fn rebuild_engine_for_current_strength(&mut self) -> Result<(), String> {
        self.engine = build_engine(self.effective_skill_level());
            self.apply_engine_options()?;
            self.engine.new_game();
        Ok(())
    }

    fn handle_position(&mut self, line: &str) -> Result<(), String> {
        let _ = self.stop_async_search_and_collect();
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
        let _ = self.stop_async_search_and_collect();
        let mut params = parse_go_params(line, &self.game_state)?;
        if params.mate.is_some() && params.depth.is_none() && params.nodes.is_none() {
            // `go mate N` should remain mate-driven and not be masked by a fixed-depth override.
            if let Some(m) = params.mate {
                params.depth = Some(m.saturating_mul(2).saturating_add(1).max(1));
            }
            writeln!(
                out,
                "info string go mate requested {:?}; using mate-priority search",
                params.mate
            )
            .map_err(|e| e.to_string())?;
        } else if params.depth.is_none() {
            params.depth = self.fixed_depth_override;
        }
        if params.mate.is_none() && (params.infinite || params.ponder) {
            let start_mode = if params.ponder { "ponder" } else { "infinite" };
            self.start_async_search(params)?;
            writeln!(
                out,
                "info string async search started mode={start_mode}; waiting for stop/ponderhit"
            )
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        let result = self.engine.choose_move(&self.game_state, &params)?;
        self.emit_engine_output(&result, out)
    }

    fn handle_stop(&mut self, out: &mut impl Write) -> Result<(), String> {
        let had_async = self.async_search.is_some();
        if let Some(result) = self.stop_async_search_and_collect()? {
            return self.emit_engine_output(&result, out);
        }
        if had_async {
            let fallback = GoParams {
                depth: self.fixed_depth_override.or(Some(2)),
                movetime_ms: Some(100),
                ..GoParams::default()
            };
            let result = self.engine.choose_move(&self.game_state, &fallback)?;
            return self.emit_engine_output(&result, out);
        }
        Ok(())
    }

    fn handle_ponderhit(&mut self, out: &mut impl Write) -> Result<(), String> {
        if let Some(active) = self.async_search.as_ref() {
            if active.is_ponder {
                let mut resumed = active.go_params.clone();
                resumed.ponder = false;
                resumed.infinite = false;
                if resumed.movetime_ms.is_none()
                    && resumed.wtime_ms.is_none()
                    && resumed.btime_ms.is_none()
                {
                    resumed.movetime_ms = Some(250);
                }
                let _ = self.stop_async_search_and_collect()?;
                self.start_async_search(resumed)?;
                writeln!(
                    out,
                    "info string ponderhit accepted; switched to normal async search"
                )
                .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
        let had_async = self.async_search.is_some();
        if let Some(result) = self.stop_async_search_and_collect()? {
            return self.emit_engine_output(&result, out);
        }
        if had_async {
            let fallback = GoParams {
                depth: self.fixed_depth_override.or(Some(2)),
                movetime_ms: Some(100),
                ..GoParams::default()
            };
            let result = self.engine.choose_move(&self.game_state, &fallback)?;
            return self.emit_engine_output(&result, out);
        }
        Ok(())
    }

    fn emit_engine_output(
        &mut self,
        result: &crate::engines::engine_trait::EngineOutput,
        out: &mut impl Write,
    ) -> Result<(), String> {
        for info in &result.info_lines {
            writeln!(out, "{}", info).map_err(|e| e.to_string())?;
        }
        if self.show_wdl {
            if let Some(cp) = extract_last_cp_score(&result.info_lines) {
                let (w, d, l) = cp_to_wdl(cp);
                writeln!(out, "info wdl {} {} {}", w, d, l).map_err(|e| e.to_string())?;
            }
        }
        if self.show_currline {
            if let Some(currline) = build_currline_text(result, &self.game_state) {
                writeln!(out, "info currline {}", currline).map_err(|e| e.to_string())?;
            }
        }

        if let Some(best_move) = result.best_move {
            let lan = move_description_to_long_algebraic(best_move, &self.game_state)?;
            if let Some(ponder_move) = result.ponder_move {
                let next_state = apply_move(&self.game_state, best_move)?;
                let mut probe = next_state.clone();
                let legal_ponder = generate_legal_move_descriptions_in_place(&mut probe)
                    .map_err(|e| e.to_string())?;
                if legal_ponder.contains(&ponder_move) {
                    if let Ok(ponder_lan) =
                        move_description_to_long_algebraic(ponder_move, &next_state)
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
                writeln!(out, "bestmove {}", lan).map_err(|e| e.to_string())?;
            }
        } else {
            writeln!(out, "bestmove 0000").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn apply_engine_options(&mut self) -> Result<(), String> {
        self.engine.set_option("Hash", &self.hash_mb.to_string())?;
        self.engine
            .set_option("Threads", &self.threads.to_string())?;
        self.engine
            .set_option("Ponder", if self.ponder { "true" } else { "false" })?;
        self.engine.set_option(
            "UCI_LimitStrength",
            if self.limit_strength { "true" } else { "false" },
        )?;
        self.engine
            .set_option("UCI_Elo", &self.uci_elo.to_string())?;
        self.engine
            .set_option("MultiPV", &self.multipv.to_string())?;
        self.engine.set_option(
            "UCI_AnalyseMode",
            if self.analyse_mode { "true" } else { "false" },
        )?;
        self.engine
            .set_option("UCI_Chess960", if self.chess960 { "true" } else { "false" })?;
        self.engine.set_option(
            "UCI_ShowCurrLine",
            if self.show_currline { "true" } else { "false" },
        )?;
        self.engine.set_option(
            "UCI_ShowRefutations",
            if self.show_refutations { "true" } else { "false" },
        )?;
        self.engine
            .set_option("UCI_Opponent", &self.uci_opponent)?;
        self.engine
            .set_option("OwnBook", if self.own_book { "true" } else { "false" })?;
        self.engine
            .set_option("TimeStrategy", &self.time_strategy)?;
        Ok(())
    }

    fn start_async_search(&mut self, params: GoParams) -> Result<(), String> {
        let is_ponder = params.ponder;
        let game_state = self.game_state.clone();
        let skill_level = self.effective_skill_level();
        let hash_mb = self.hash_mb;
        let threads = self.threads;
        let own_book = self.own_book;
        let ponder = self.ponder;
        let limit_strength = self.limit_strength;
        let uci_elo = self.uci_elo;
        let multipv = self.multipv;
        let analyse_mode = self.analyse_mode;
        let chess960 = self.chess960;
        let show_currline = self.show_currline;
        let show_refutations = self.show_refutations;
        let uci_opponent = self.uci_opponent.clone();
        let time_strategy = self.time_strategy.clone();
        let info_tx = self.async_info_tx.clone();
        let depth_override = self.fixed_depth_override;
        let params_for_worker = params.clone();

        let stop = Arc::new(AtomicBool::new(false));
        let latest = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let stop_flag = Arc::clone(&stop);
        let latest_ref = Arc::clone(&latest);
        let error_ref = Arc::clone(&error);

        let handle = thread::spawn(move || {
            let mut worker_engine = build_engine(skill_level);
            worker_engine.set_stop_signal(Some(Arc::clone(&stop_flag)));
            let _ = worker_engine.set_option("Hash", &hash_mb.to_string());
            let _ = worker_engine.set_option("Threads", &threads.to_string());
            let _ = worker_engine.set_option("OwnBook", if own_book { "true" } else { "false" });
            let _ = worker_engine.set_option("Ponder", if ponder { "true" } else { "false" });
            let _ = worker_engine.set_option(
                "UCI_LimitStrength",
                if limit_strength { "true" } else { "false" },
            );
            let _ = worker_engine.set_option("UCI_Elo", &uci_elo.to_string());
            let _ = worker_engine.set_option("MultiPV", &multipv.to_string());
            let _ = worker_engine.set_option(
                "UCI_AnalyseMode",
                if analyse_mode { "true" } else { "false" },
            );
            let _ =
                worker_engine.set_option("UCI_Chess960", if chess960 { "true" } else { "false" });
            let _ = worker_engine.set_option(
                "UCI_ShowCurrLine",
                if show_currline { "true" } else { "false" },
            );
            let _ = worker_engine.set_option(
                "UCI_ShowRefutations",
                if show_refutations { "true" } else { "false" },
            );
            let _ = worker_engine.set_option("UCI_Opponent", &uci_opponent);
            let _ = worker_engine.set_option("TimeStrategy", &time_strategy);
            worker_engine.new_game();

            let mut iter_depth = 1u8;
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                let mut iter_params = params_for_worker.clone();
                iter_params.ponder = false;
                iter_params.infinite = false;
                if iter_params.movetime_ms.is_none() {
                    iter_params.movetime_ms = Some(75);
                }

                if iter_params.depth.is_none() {
                    iter_params.depth = depth_override.or(Some(iter_depth));
                    if depth_override.is_none() {
                        iter_depth = iter_depth.saturating_add(1).min(32);
                    }
                }

                match worker_engine.choose_move(&game_state, &iter_params) {
                    Ok(out) => {
                        if let Some(tx) = &info_tx {
                            for line in &out.info_lines {
                                let _ = tx.send(line.clone());
                            }
                            if show_currline {
                                if let Some(currline) = build_currline_text(&out, &game_state) {
                                    let _ = tx.send(format!("info currline {}", currline));
                                }
                            }
                            if let Some(best) = out.best_move {
                                if let Ok(curr_lan) =
                                    move_description_to_long_algebraic(best, &game_state)
                                {
                                    let _ = tx.send(format!(
                                        "info currmove {} currmovenumber {}",
                                        curr_lan, iter_depth
                                    ));
                                }
                            }
                        }
                        if let Ok(mut guard) = latest_ref.lock() {
                            *guard = Some(out);
                        }
                    }
                    Err(e) => {
                        if let Ok(mut guard) = error_ref.lock() {
                            *guard = Some(e);
                        }
                        break;
                    }
                }
            }
        });

        self.async_search = Some(AsyncSearchHandle {
            stop,
            latest,
            error,
            go_params: params,
            is_ponder,
            handle,
        });
        Ok(())
    }

    fn stop_async_search_and_collect(
        &mut self,
    ) -> Result<Option<crate::engines::engine_trait::EngineOutput>, String> {
        let Some(async_handle) = self.async_search.take() else {
            return Ok(None);
        };
        async_handle.stop.store(true, Ordering::Relaxed);
        async_handle
            .handle
            .join()
            .map_err(|_| "async search thread panicked".to_owned())?;

        if let Ok(mut err_guard) = async_handle.error.lock() {
            if let Some(err) = err_guard.take() {
                return Err(err);
            }
        }
        if let Ok(mut latest_guard) = async_handle.latest.lock() {
            return Ok(latest_guard.take());
        }
        Err("failed to read async search result".to_owned())
    }
}

fn extract_last_cp_score(info_lines: &[String]) -> Option<i32> {
    for line in info_lines.iter().rev() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        for w in tokens.windows(2) {
            if w[0] == "cp" {
                if let Ok(v) = w[1].parse::<i32>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn extract_last_pv_moves(info_lines: &[String]) -> Option<String> {
    for line in info_lines.iter().rev() {
        if let Some(rest) = line.strip_prefix("info pv ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

fn build_currline_text(
    result: &crate::engines::engine_trait::EngineOutput,
    game_state: &GameState,
) -> Option<String> {
    if let Some(pv) = extract_last_pv_moves(&result.info_lines) {
        return Some(pv);
    }
    if let Some(best) = result.best_move {
        if let Ok(best_lan) = move_description_to_long_algebraic(best, game_state) {
            return Some(best_lan);
        }
    }
    None
}

fn cp_to_wdl(cp: i32) -> (u16, u16, u16) {
    let cp_f = cp as f64;
    let win_sigmoid = 1.0 / (1.0 + (-cp_f / 180.0).exp());
    let draw = (0.30 - (cp_f.abs() / 1200.0)).clamp(0.05, 0.30);
    let decisive = 1.0 - draw;
    let win = decisive * win_sigmoid;
    let loss = decisive * (1.0 - win_sigmoid);

    let mut w = (win * 1000.0).round() as i32;
    let mut d = (draw * 1000.0).round() as i32;
    let mut l = (loss * 1000.0).round() as i32;
    let delta = 1000 - (w + d + l);
    if delta > 0 {
        d += delta;
    } else if delta < 0 {
        let remove = (-delta).min(d);
        d -= remove;
    }
    w = w.clamp(0, 1000);
    d = d.clamp(0, 1000);
    l = (1000 - w - d).clamp(0, 1000);
    (w as u16, d as u16, l as u16)
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

fn elo_to_skill_level(elo: u16) -> u8 {
    match elo {
        0..=699 => 1,
        700..=799 => 2,
        800..=899 => 3,
        900..=999 => 4,
        1000..=1099 => 5,
        1100..=1199 => 6,
        1200..=1299 => 7,
        1300..=1399 => 8,
        1400..=1499 => 9,
        1500..=1599 => 10,
        1600..=1699 => 11,
        1700..=1749 => 12,
        1750..=1799 => 13,
        _ => 14,
    }
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
        11 => Box::new(IterativeEngine::new_standard(7)),
        12 => Box::new(IterativeEngine::new_alpha_zero(7)),
        13 => Box::new(IterativeEngine::new_standard(9)),
        14 => Box::new(IterativeEngine::new_alpha_zero(9)),
        15 => Box::new(IterativeEngine::new_standard(11)),
        16 => Box::new(IterativeEngine::new_alpha_zero(11)),
        17 => Box::new(IterativeEngine::new_standard(13)),
        18 => Box::new(IterativeEngine::new_alpha_zero(13)),
        19 => Box::new(IterativeEngine::new_alpha_zero(15)),
        _ => Box::new(IterativeEngine::new_alpha_zero(20)),
    }
}

#[cfg(test)]
mod tests {
    use super::{elo_to_skill_level, UciState};

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
    fn setoption_limit_strength_and_elo_parse_and_clamp() {
        let mut state = UciState::new();
        assert!(!state.limit_strength);
        assert_eq!(state.uci_elo, 1200);

        state
            .handle_setoption("setoption name UCI_LimitStrength value true")
            .expect("limit strength should parse");
        state
            .handle_setoption("setoption name UCI_Elo value 1900")
            .expect("uci elo should parse");

        assert!(state.limit_strength);
        assert_eq!(state.uci_elo, 1800);
        assert_eq!(state.effective_skill_level(), 14);
    }

    #[test]
    fn setoption_multipv_parse_and_clamp() {
        let mut state = UciState::new();
        assert_eq!(state.multipv, 1);
        state
            .handle_setoption("setoption name MultiPV value 4")
            .expect("multipv should parse");
        assert_eq!(state.multipv, 4);
        state
            .handle_setoption("setoption name MultiPV value 99")
            .expect("multipv clamp should parse");
        assert_eq!(state.multipv, 32);
    }

    #[test]
    fn elo_to_skill_mapping_tracks_current_engine_range() {
        assert_eq!(elo_to_skill_level(600), 1);
        assert_eq!(elo_to_skill_level(900), 4);
        assert_eq!(elo_to_skill_level(1200), 7);
        assert_eq!(elo_to_skill_level(1500), 10);
        assert_eq!(elo_to_skill_level(1700), 12);
        assert_eq!(elo_to_skill_level(1800), 14);
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
    fn setoption_clear_hash_button_is_accepted() {
        let mut state = UciState::new();
        state
            .handle_setoption("setoption name Clear Hash")
            .expect("clear hash should parse");
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
    fn setoption_show_currline_and_refutations_parse() {
        let mut state = UciState::new();
        assert!(!state.show_currline);
        assert!(!state.show_refutations);
        state
            .handle_setoption("setoption name UCI_ShowCurrLine value true")
            .expect("show currline should parse");
        state
            .handle_setoption("setoption name UCI_ShowRefutations value true")
            .expect("show refutations should parse");
        assert!(state.show_currline);
        assert!(state.show_refutations);
    }

    #[test]
    fn setoption_uci_opponent_parse() {
        let mut state = UciState::new();
        state
            .handle_setoption("setoption name UCI_Opponent value GM 2800 human Gary Kasparov")
            .expect("uci opponent should parse");
        assert_eq!(state.uci_opponent, "GM 2800 human Gary Kasparov");
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

    #[test]
    fn go_mate_priority_overrides_fixed_depth_hint() {
        let mut state = UciState::new();
        state.fixed_depth_override = Some(1);
        state
            .handle_setoption("setoption name Skill Level value 3")
            .expect("setoption should parse");
        state
            .handle_setoption("setoption name OwnBook value false")
            .expect("setoption should parse");

        let mut out = Vec::<u8>::new();
        state
            .handle_command("go mate 2", &mut out)
            .expect("go mate should work");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("go mate requested"));
        assert!(text.contains("mate_mode plies_target 5"));
        assert!(text.contains("bestmove"));
    }

    #[test]
    fn go_mate_ignores_infinite_mode_and_returns_bestmove() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("go infinite mate 1", &mut out)
            .expect("go mate should be handled synchronously");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("go mate requested"));
        assert!(text.contains("bestmove"));
        assert!(!text.contains("async search started"));
        assert!(state.async_search.is_none());
    }

    #[test]
    fn go_infinite_defers_until_stop() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        let _ = state
            .handle_command("go infinite", &mut out)
            .expect("go infinite should parse");
        let text = String::from_utf8(out.clone()).expect("valid utf8");
        assert!(text.contains("async search started"));
        assert!(!text.contains("bestmove"));

        out.clear();
        let _ = state
            .handle_command("stop", &mut out)
            .expect("stop should parse");
        let text = String::from_utf8(out).expect("valid utf8");
        assert!(text.contains("bestmove"));
    }

    #[test]
    fn uci_state_accepts_async_info_sender() {
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let mut state = UciState::new();
        state.set_async_info_sender(Some(tx));
        assert!(state.async_info_tx.is_some());
        state.set_async_info_sender(None);
        assert!(state.async_info_tx.is_none());
    }

    #[test]
    fn go_infinite_stop_is_stable_over_multiple_cycles() {
        let mut state = UciState::new();
        for _ in 0..5 {
            let mut out = Vec::<u8>::new();
            state
                .handle_command("go infinite", &mut out)
                .expect("go infinite should succeed");
            let start_text = String::from_utf8(out).expect("valid utf8");
            assert!(start_text.contains("async search started"));

            let mut stop_out = Vec::<u8>::new();
            state
                .handle_command("stop", &mut stop_out)
                .expect("stop should succeed");
            let stop_text = String::from_utf8(stop_out).expect("valid utf8");
            assert!(stop_text.contains("bestmove"));
            assert!(state.async_search.is_none());
        }
    }

    #[test]
    fn setoption_stops_active_async_search() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("go infinite", &mut out)
            .expect("go infinite should succeed");
        assert!(state.async_search.is_some());

        let mut set_out = Vec::<u8>::new();
        state
            .handle_command("setoption name Hash value 128", &mut set_out)
            .expect("setoption should succeed");
        assert!(state.async_search.is_none());
    }

    #[test]
    fn ponderhit_switches_ponder_to_normal_async_without_bestmove() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("go ponder", &mut out)
            .expect("go ponder should succeed");
        let text = String::from_utf8(out).expect("valid utf8");
        assert!(text.contains("mode=ponder"));
        assert!(state
            .async_search
            .as_ref()
            .map(|h| h.is_ponder)
            .unwrap_or(false));

        let mut hit_out = Vec::<u8>::new();
        state
            .handle_command("ponderhit", &mut hit_out)
            .expect("ponderhit should succeed");
        let hit_text = String::from_utf8(hit_out).expect("valid utf8");
        assert!(hit_text.contains("ponderhit accepted"));
        assert!(!hit_text.contains("bestmove"));
        assert!(state.async_search.is_some());
        assert!(!state
            .async_search
            .as_ref()
            .map(|h| h.is_ponder)
            .unwrap_or(true));
    }

    #[test]
    fn ponder_stop_emits_bestmove() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("go ponder", &mut out)
            .expect("go ponder should succeed");
        out.clear();
        state
            .handle_command("stop", &mut out)
            .expect("stop should succeed");
        let text = String::from_utf8(out).expect("valid utf8");
        assert!(text.contains("bestmove"));
    }

    #[test]
    fn startup_sequence_is_resilient() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("uci", &mut out)
            .expect("uci should work");
        let uci_text = String::from_utf8(out.clone()).expect("utf8");
        assert!(uci_text.contains("uciok"));
        assert!(uci_text.contains("UCI_LimitStrength"));
        assert!(uci_text.contains("UCI_Elo"));
        assert!(uci_text.contains("MultiPV"));
        assert!(uci_text.contains("UCI_ShowWDL"));
        assert!(uci_text.contains("UCI_ShowCurrLine"));
        assert!(uci_text.contains("UCI_ShowRefutations"));
        assert!(uci_text.contains("UCI_Opponent"));
        assert!(uci_text.contains("UCI_EngineAbout"));

        out.clear();
        state
            .handle_command("isready", &mut out)
            .expect("isready should work");
        assert!(String::from_utf8(out.clone())
            .expect("utf8")
            .contains("readyok"));

        out.clear();
        state
            .handle_command("setoption name Hash value 128", &mut out)
            .expect("setoption should work");
        state
            .handle_command("setoption name Threads value 2", &mut out)
            .expect("setoption should work");
        state
            .handle_command("setoption name UCI_ShowWDL value true", &mut out)
            .expect("setoption should work");
        state
            .handle_command("ucinewgame", &mut out)
            .expect("ucinewgame should work");
        state
            .handle_command("position startpos moves e2e4 e7e5", &mut out)
            .expect("position should work");
        out.clear();
        state
            .handle_command("go depth 1", &mut out)
            .expect("go should work");
        let go_text = String::from_utf8(out).expect("utf8");
        assert!(go_text.contains("bestmove"));
    }

    #[test]
    fn malformed_position_reports_error_and_keeps_loop_alive() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("position fen", &mut out)
            .expect("command loop should not fail");
        let text = String::from_utf8(out.clone()).expect("utf8");
        assert!(text.contains("position error"));

        out.clear();
        state
            .handle_command("isready", &mut out)
            .expect("isready should still work");
        assert!(String::from_utf8(out).expect("utf8").contains("readyok"));
    }

    #[test]
    fn isready_responds_during_async_search() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("go infinite", &mut out)
            .expect("go infinite should start");
        out.clear();
        state
            .handle_command("isready", &mut out)
            .expect("isready should respond while searching");
        assert!(String::from_utf8(out).expect("utf8").contains("readyok"));
        let mut stop_out = Vec::<u8>::new();
        state
            .handle_command("stop", &mut stop_out)
            .expect("stop should succeed");
    }

    #[test]
    fn show_wdl_outputs_wdl_info_line() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("setoption name OwnBook value false", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name Skill Level value 3", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name UCI_ShowWDL value true", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("go depth 1", &mut out)
            .expect("go should succeed");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("info wdl"));
    }

    #[test]
    fn show_refutations_outputs_refutation_info_line() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("setoption name OwnBook value false", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name Skill Level value 3", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name UCI_ShowRefutations value true", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("go depth 1", &mut out)
            .expect("go should succeed");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("info refutation "));
    }

    #[test]
    fn show_currline_outputs_currline_info_line() {
        let mut state = UciState::new();
        let mut out = Vec::<u8>::new();
        state
            .handle_command("setoption name OwnBook value false", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name Skill Level value 3", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("setoption name UCI_ShowCurrLine value true", &mut out)
            .expect("setoption should parse");
        out.clear();
        state
            .handle_command("go depth 1", &mut out)
            .expect("go should succeed");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("info currline "));
    }
}
