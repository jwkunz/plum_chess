use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self},
    time::Duration,
};

use crate::{apply_move_to_game::apply_move_to_game_unchecked, chess_engine_thread_trait::{ChessEngineThreadTrait, EngineControlMessageType, EngineResponseMessageType}, chess_errors::ChessErrors, engine_random::EngineRandom, game_state::GameState, move_description::MoveDescription};


/// Tokens for setting position values in UCI options.
#[derive(Debug)]
enum UCISetPositionValueTokens {
    /// Set a value (name, value).
    Value((String, String)),
    /// Clear a named value.
    Clear(String),
    /// Clear all values.
    ClearAll,
}

/// Types of UCI options.
#[derive(Debug)]
enum OptionTypeToken {
    Check,
    Spin,
    Combo,
    Button,
    String,
}

/// Describes a property of a UCI option (e.g., default, min, max).
#[derive(Debug)]
enum OptionDescriptionToken {
    Default,
    Min,
    Max,
    Var,
}

/// Represents all possible UCI options supported by the engine.
#[derive(Debug)]
enum OptionToken {
    Hash(u32),
    NalimovePath(String),
    NalimoveCache(u32),
    Ponder(bool),
    OwnBook(bool),
    MultiPV(u32),
    UCIShowCurrentLine(bool),
    UCIShowRefutations(bool),
    UCILimitStrength(bool),
    UCIELO(u32),
    UCIAnalyzeMode(bool),
    UCIOpponent(String),
    UCIEngineAbout(String),
    UCIShredderBasesPath(String),
    UCISetPositionValue(String),
}

/// Represents a register command for UCI.
#[derive(Debug)]
enum RegisterToken {
    Value(String),
}

/// Represents a position token: either a FEN string or the standard starting position.
#[derive(Debug)]
enum PositionToken {
    Fen(String),
    StartPosition,
}

/// Represents all possible tokens for the UCI "go" command.
#[derive(Debug)]
enum GoTokens {
    SearchMoves(Vec<String>),
    Ponder,
    Wtime(f32),
    Btime(f32),
    Winc(f32),
    Binc(f32),
    MovesToGo(u8),
    Depth(u8),
    Nodes(u8),
    Mate(u8),
    MoveTime(f32),
    Infinite,
}

/// Represents all possible UCI commands as tokens.
#[derive(Debug)]
enum CommandTokens {
    Uci,
    Debug(bool),
    IsReady,
    SetOption(OptionToken),
    Register(RegisterToken),
    UciNewGame,
    Position((PositionToken, Vec<String>)),
    Go(GoTokens),
    Stop,
    PonderHit,
    Quit,
}

/// Parses a UCI command string into a vector of command tokens.
///
/// # Arguments
/// * `input` - The input string from the UCI protocol.
///
/// # Returns
/// * `Vec<CommandTokens>` - The parsed command tokens.
fn parse_command(input: &str) -> Vec<CommandTokens> {
    let mut tokens = Vec::new();
    let mut words = input.split_whitespace().peekable();

    while let Some(word) = words.next() {
        match word {
            "uci" => tokens.push(CommandTokens::Uci),
            "isready" => tokens.push(CommandTokens::IsReady),
            "ucinewgame" => tokens.push(CommandTokens::UciNewGame),
            "stop" => tokens.push(CommandTokens::Stop),
            "ponderhit" => tokens.push(CommandTokens::PonderHit),
            "quit" => tokens.push(CommandTokens::Quit),
            "debug" => {
                let on_off = words
                    .next()
                    .map(|w| w.eq_ignore_ascii_case("on"))
                    .unwrap_or(false);
                tokens.push(CommandTokens::Debug(on_off));
            }
            "setoption" => {
                // Example: setoption name Hash value 32
                let mut name = String::new();
                let mut value = String::new();
                while let Some(next) = words.peek() {
                    match *next {
                        "name" => {
                            words.next();
                            // Collect name until "value" or end
                            let mut name_parts = Vec::new();
                            while let Some(&w) = words.peek() {
                                if w == "value" {
                                    break;
                                }
                                name_parts.push(words.next().unwrap());
                            }
                            name = name_parts.join(" ");
                        }
                        "value" => {
                            words.next();
                            let mut value_parts = Vec::new();
                            while let Some(&w) = words.peek() {
                                value_parts.push(words.next().unwrap());
                            }
                            value = value_parts.join(" ");
                        }
                        _ => {
                            words.next();
                        }
                    }
                }
                if let Some(out_token) = match name.as_str() {
                    "UCI_LimitStrength" => {
                        if let Some(x) = match value.as_str() {
                            "true" => Some(true),
                            "false" => Some(false),
                            _ => None,
                        } {
                            Some(OptionToken::UCILimitStrength(x))
                        } else {
                            None
                        }
                    }
                    "UCI_Elo" => {
                        if let Ok(x) = value.parse::<u32>() {
                            Some(OptionToken::UCIELO(x))
                        } else {
                            None
                        }
                    }
                    _ => None,
                } {
                    tokens.push(CommandTokens::SetOption(out_token));
                }
            }
            "register" => {
                // TODO: Parse register tokens
                tokens.push(CommandTokens::Register(RegisterToken::Value(
                    " ".to_string(),
                )));
            }
            "position" => {
                // Example: position startpos moves e2e4 e7e5
                let mut fen = String::new();
                let mut moves = Vec::new();
                let mut position_token = PositionToken::StartPosition;
                if let Some(next) = words.peek() {
                    if *next == "startpos" {
                        words.next();
                    } else if *next == "fen" {
                        words.next();
                        // Collect FEN string (6 fields)
                        let fen_fields: Vec<_> = words.by_ref().take(6).collect();
                        fen = fen_fields.join(" ");
                        position_token = PositionToken::Fen(fen);
                    }
                }
                // Look for "moves"
                if let Some(&"moves") = words.peek() {
                    words.next();
                    while let Some(mv) = words.next() {
                        moves.push(mv.to_string());
                    }
                    tokens.push(CommandTokens::Position((position_token, moves)));
                }
            }
            "go" => {
                // TODO: Parse go tokens
                tokens.push(CommandTokens::Go(GoTokens::Infinite));
            }
            _ => {
                // Unknown command, ignore or handle as needed
            }
        }
    }

    tokens
}

/// Represents the engine's ID tokens for UCI responses.
enum IDTokens {
    Name(String),
    Author(String),
}

/// Represents copy protection status for UCI.
enum CopyProtectionToken {
    Ok,
    Error,
}

/// Represents registration status for UCI.
enum RegistrationToken {
    Ok,
    Error,
    Checking,
}

/// Represents a score in UCI info output.
enum Score {
    CP(f32),
    Mate(u8),
    LowerBound,
    UpperBound,
}

/// Represents all possible info tokens for UCI "info" responses.
enum InfoToken {
    Depth(u16),
    SelectionDepth(u16),
    Time(f32),
    Nodes(u32),
    NPS(u64),
    PV(Vec<String>),
    MultiPV(u16),
    Score,
    CurrentMove(String),
    CurrentMoveNumber(u16),
    HashFull(u16),
    TableHits(u16),
    ShredderHits(u16),
    CpuLoad(f32),
    String(String),
    Refutation(Vec<String>),
    CurrentLine(Vec<String>, Vec<String>),
}

/// Represents all possible UCI responses as tokens.
enum ResponseTokens {
    ID(IDTokens),
    UciOK,
    ReadyOk,
    BestMove(String),
    Copyprotection(CopyProtectionToken),
    Registration(RegistrationToken),
    Info(InfoToken),
    Option(OptionToken),
}

/// Converts a response token to its corresponding UCI protocol string.
///
/// # Arguments
/// * `token` - The response token to convert.
///
/// # Returns
/// * `String` - The UCI protocol string.
fn generate_response(token: ResponseTokens) -> String {
    match token {
        ResponseTokens::ID(id_token) => match id_token {
            IDTokens::Name(name) => format!("id name {}", name),
            IDTokens::Author(author) => format!("id author {}", author),
        },
        ResponseTokens::UciOK => "uciok".to_string(),
        ResponseTokens::ReadyOk => "readyok".to_string(),
        ResponseTokens::BestMove(mv) => format!("bestmove {}", mv),
        ResponseTokens::Copyprotection(cp_token) => match cp_token {
            CopyProtectionToken::Ok => "copyprotection ok".to_string(),
            CopyProtectionToken::Error => "copyprotection error".to_string(),
        },
        ResponseTokens::Registration(reg_token) => match reg_token {
            RegistrationToken::Ok => "registration ok".to_string(),
            RegistrationToken::Error => "registration error".to_string(),
            RegistrationToken::Checking => "registration checking".to_string(),
        },
        ResponseTokens::Info(info_token) => match info_token {
            InfoToken::Depth(d) => format!("info depth {}", d),
            InfoToken::SelectionDepth(sd) => format!("info seldepth {}", sd),
            InfoToken::Time(t) => format!("info time {}", t as u64),
            InfoToken::Nodes(n) => format!("info nodes {}", n),
            InfoToken::NPS(nps) => format!("info nps {}", nps),
            InfoToken::PV(pv) => format!("info pv {}", pv.join(" ")),
            InfoToken::MultiPV(n) => format!("info multipv {}", n),
            InfoToken::Score => "info score".to_string(),
            InfoToken::CurrentMove(mv) => format!("info currmove {}", mv),
            InfoToken::CurrentMoveNumber(n) => format!("info currmovenumber {}", n),
            InfoToken::HashFull(h) => format!("info hashfull {}", h),
            InfoToken::TableHits(h) => format!("info tbhits {}", h),
            InfoToken::ShredderHits(h) => format!("info sbhits {}", h),
            InfoToken::CpuLoad(load) => format!("info cpuload {}", load),
            InfoToken::String(s) => format!("info string {}", s),
            InfoToken::Refutation(moves) => format!("info refutation {}", moves.join(" ")),
            InfoToken::CurrentLine(cpu, moves) => {
                if cpu.is_empty() {
                    format!("info currline {}", moves.join(" "))
                } else {
                    format!("info currline {} {}", cpu.join(" "), moves.join(" "))
                }
            }
        },
        ResponseTokens::Option(option_token) => match option_token {
            OptionToken::Hash(val) => {
                format!("option name Hash type spin default {} min 1 max 128", val)
            }
            OptionToken::NalimovePath(path) => {
                format!("option name NalimovPath type string default {}", path)
            }
            OptionToken::NalimoveCache(val) => format!(
                "option name NalimovCache type spin default {} min 1 max 32",
                val
            ),
            OptionToken::Ponder(val) => {
                format!("option name Ponder type check default {}", val)
            }
            OptionToken::OwnBook(val) => {
                format!("option name OwnBook type check default {}", val)
            }
            OptionToken::MultiPV(val) => {
                format!("option name MultiPV type spin default {}", val)
            }
            OptionToken::UCIShowCurrentLine(val) => {
                format!("option name UCI_ShowCurrLine type check default {}", val)
            }
            OptionToken::UCIShowRefutations(val) => {
                format!("option name UCI_ShowRefutations type check default {}", val)
            }
            OptionToken::UCILimitStrength(val) => {
                format!("option name UCI_LimitStrength type check default {}", val)
            }
            OptionToken::UCIELO(val) => {
                format!("option name UCI_Elo type spin default {} min 1 max 20", val)
            }
            OptionToken::UCIAnalyzeMode(val) => {
                format!("option name UCI_AnalyseMode type check default {}", val)
            }
            OptionToken::UCIOpponent(s) => {
                format!("option name UCI_Opponent type string default {}", s)
            }
            OptionToken::UCIEngineAbout(s) => {
                format!("option name UCI_EngineAbout type string default {}", s)
            }
            OptionToken::UCIShredderBasesPath(s) => format!(
                "option name UCI_ShredderbasesPath type string default {}",
                s
            ),
            OptionToken::UCISetPositionValue(s) => {
                format!("option name UCI_SetPositionValue type string default {}", s)
            }
        },
    }
}

/// Represents the internal state of the UCI protocol handler.
#[derive(Clone, Copy, Debug)]
enum UCIstate {
    Startup,
    WaitStartup,
    WaitBootComplete,
    Idle,
    MonitorCalculation,
}

/// The main UCI protocol handler struct.
/// Manages the state machine, communication channels, and current analysis position.
pub struct UCI {
    /// The current state of the UCI protocol handler.
    uci_state: UCIstate,
    /// Channel for receiving commands from the main thread.
    command_rx: Receiver<String>,
    /// Channel for sending responses to the main thread.
    response_tx: Sender<String>,
    /// The current position to analyze, if any.
    position_to_analyze: Option<GameState>,
    // Options
    uci_limit_strength: bool,
    uci_elo: u32,
    // Engine IO
    command_sender: Option<mpsc::Sender<EngineControlMessageType>>,
    response_receiver: Option<mpsc::Receiver<EngineResponseMessageType>>,
    run_engine_thread: Option<Arc<AtomicBool>>,
}

impl UCI {
    /// Creates a new UCI protocol handler.
    ///
    /// # Arguments
    /// * `command_rx` - Receiver for incoming commands.
    /// * `response_tx` - Sender for outgoing responses.
    ///
    /// # Returns
    /// * `Self` - The initialized UCI handler.
    pub fn new(command_rx: Receiver<String>, response_tx: Sender<String>) -> Self {
        UCI {
            uci_state: UCIstate::Startup,
            command_rx,
            response_tx,
            position_to_analyze: None,
            command_sender: None,
            response_receiver: None,
            run_engine_thread: None,
            uci_limit_strength: true,
            uci_elo: 20,
        }
    }

    /// Advances the UCI protocol state machine by one tick.
    /// Handles incoming commands, state transitions, and engine actions.
    pub fn tick(&mut self) {
        // Next state logic
        let next_state = match self.uci_state {
            // Peform startup actions
            UCIstate::Startup => {
                if let Some(x) = self.get_command() {
                    let commands = parse_command(&x);
                    match commands.first() {
                        None => self.uci_state,
                        Some(token) => match token {
                            CommandTokens::Uci => {
                                self.respond_to_uci_init();
                                self.launch_boot_actions();
                                UCIstate::WaitStartup
                            }
                            _ => self.uci_state,
                        },
                    }
                } else {
                    self.uci_state
                }
            }

            // Waiting for external to send options and/or "is_ready"
            UCIstate::WaitStartup => {
                if let Some(x) = self.get_command() {
                    let commands = parse_command(&x);
                    match commands.first() {
                        None => self.uci_state,
                        Some(token) => match token {
                            CommandTokens::SetOption(opt) => {
                                self.set_options(opt);
                                self.uci_state
                            }
                            CommandTokens::IsReady => UCIstate::WaitBootComplete,
                            CommandTokens::Quit => {
                                self.quit_cleanup();
                                UCIstate::Startup
                            }
                            _ => self.uci_state,
                        },
                    }
                } else {
                    self.uci_state
                }
            }

            // Check if the boot actions thread is done
            UCIstate::WaitBootComplete => {
                if self.are_boot_actions_done() {
                    // Tell external that all is done
                    self.give_response(generate_response(ResponseTokens::ReadyOk));
                    UCIstate::Idle
                } else {
                    // Wait for boot thread to finish

                    // Sleep briefly to avoid busy-waiting
                    thread::sleep(Duration::from_millis(10));
                    self.uci_state
                }
            }

            // Waiting for a command
            UCIstate::Idle => {
                if let Some(x) = self.get_command() {
                    let commands = parse_command(&x);
                    match commands.first() {
                        None => self.uci_state,
                        Some(token) => match token {
                            CommandTokens::UciNewGame => {
                                // TODO
                                self.uci_state
                            }
                            CommandTokens::SetOption(opt) => {
                                self.set_options(opt);
                                self.uci_state
                            }
                            CommandTokens::Position((position, moves)) => {
                                let _ = self.setup_position(position, moves);
                                self.uci_state
                            }
                            CommandTokens::Go(go) => {
                                // TODO handle all tokens not just one
                                self.go_launch_calculate(go);
                                UCIstate::MonitorCalculation
                            }
                            CommandTokens::Quit => {
                                self.quit_cleanup();
                                UCIstate::Startup
                            }
                            _ => self.uci_state,
                        },
                    }
                } else {
                    self.uci_state
                }
            }

            // Monitors the calculation and handles user commands
            UCIstate::MonitorCalculation => {
                if let Some(x) = self.get_command() {
                    let commands = parse_command(&x);
                    match commands.first() {
                        None => self.uci_state,
                        Some(token) => match token {
                            CommandTokens::Stop => {
                                self.force_stop_calculate();
                                UCIstate::Idle
                            }
                            CommandTokens::Quit => {
                                self.quit_cleanup();
                                UCIstate::Startup
                            }
                            _ => self.uci_state,
                        },
                    }
                } else {
                    if self.attend_to_engine_see_if_done() {
                        self.force_stop_calculate();
                        UCIstate::Idle
                    } else {
                        self.uci_state
                    }
                }
            }
        };

        // State updated
        self.uci_state = next_state;
        // Sleep to avoid busy waiting
        thread::sleep(Duration::from_millis(10));
    }

    /// Gets a command from the input
    fn get_command(&self) -> Option<String> {
        self.command_rx.try_recv().ok()
    }

    /// Gives a response to the output
    fn give_response(&self, response: String) {
        let _ = self.response_tx.send(response);
    }

    /// Use this for launching a thread with boot-up actions
    fn launch_boot_actions(&mut self) {}

    /// Check if boot action thread is done
    fn are_boot_actions_done(&self) -> bool {
        true
    }

    /// Handler for set_options
    fn set_options(&mut self, opt: &OptionToken) {
        match opt {
            OptionToken::UCILimitStrength(x) => {
                self.uci_limit_strength = *x;
            }
            OptionToken::UCIELO(x) => {
                self.uci_elo = *x;
            }
            _ => (),
        }
    }

    /// Use this for cleaning up state during a quite
    fn quit_cleanup(&mut self) {}

    /// Gives the name and author
    fn give_name_and_author(&mut self) {
        self.give_response(generate_response(ResponseTokens::ID(IDTokens::Name(
            "Plum Chess".into(),
        ))));
        self.give_response(generate_response(ResponseTokens::ID(IDTokens::Author(
            "jwkunz".into(),
        ))));
    }

    /// Gives changeable options
    fn give_options_that_can_change(&mut self) {
        self.give_response(generate_response(ResponseTokens::Option(
            OptionToken::UCILimitStrength(self.uci_limit_strength),
        )));
        self.give_response(generate_response(ResponseTokens::Option(
            OptionToken::UCIELO(self.uci_elo),
        )));
        self.give_response(generate_response(ResponseTokens::UciOK));
    }

    /// Initial response on bootup
    fn respond_to_uci_init(&mut self) {
        self.give_name_and_author();
        self.give_options_that_can_change();
    }

    /// Setup a position
    fn setup_position(
        &mut self,
        position: &PositionToken,
        moves: &Vec<String>,
    ) -> Result<(), ChessErrors> {
        let mut game: GameState;
        match position {
            PositionToken::Fen(x) => {
                game = GameState::from_fen(x)?;
            }
            // Parse all the moves
            PositionToken::StartPosition => {
                game = GameState::new_game();
            }
        }

        for move_description in moves {
            if let Ok(m) = MoveDescription::from_long_algebraic(move_description, &game) {
                game = apply_move_to_game_unchecked(&m,&game)?;
            } else {
                self.position_to_analyze = None;
                return Err(ChessErrors::InvalidAlgebraicString(move_description.clone()));
            }
        }

        self.position_to_analyze = Some(game);
        Ok(())
    }

    /// GO command
    fn go_launch_calculate(&mut self, _go: &GoTokens) {
        if let Some(game) = &self.position_to_analyze {
            let (command_sender, command_receiver) = mpsc::channel::<EngineControlMessageType>();
            let (response_sender, response_receiver) = mpsc::channel::<EngineResponseMessageType>();
            self.command_sender = Some(command_sender);
            self.response_receiver = Some(response_receiver);
            let calculation_time = 1.0;
            let mut engine = if self.uci_limit_strength && self.uci_elo == 1 {
                self.create_engine_1(
                    game.clone(),
                    calculation_time,
                    command_receiver,
                    response_sender,
                )
            }else if self.uci_limit_strength && self.uci_elo == 2 {
                self.create_engine_2(
                    game.clone(),
                    calculation_time,
                    command_receiver,
                    response_sender,
                )
            }/*else if self.uci_limit_strength && self.uci_elo == 3 {
                self.create_engine_3(
                    game.clone(),
                    calculation_time,
                    command_receiver,
                    response_sender,
                )
            }else if self.uci_limit_strength && self.uci_elo == 4 {
                self.create_engine_4(
                    game.clone(),
                    calculation_time,
                    command_receiver,
                    response_sender,
                )
            }*/else {
                self.create_best_engine(
                    game.clone(),
                    calculation_time,
                    command_receiver,
                    response_sender,
                )
            };

            let run_engine_thread = Arc::new(AtomicBool::new(true));
            self.run_engine_thread = Some(run_engine_thread.clone());
            let _ = thread::spawn(move || {
                while run_engine_thread.load(Ordering::Relaxed) {
                    engine.tick();
                }
            });
            let _ = self
                .command_sender
                .as_ref()
                .expect("")
                .send(EngineControlMessageType::StartCalculating);
        }
    }

    /// Polls the calculate and checks if done
    fn attend_to_engine_see_if_done(&mut self) -> bool {
        let mut done_status = false;
        let response_timeout_ms = Duration::from_millis(5000);
        if let Some(cs) = &self.command_sender {
            if let Some(rr) = &self.response_receiver {
                let _ = cs.send(EngineControlMessageType::AreYouStillCalculating);
                match rr.recv_timeout(response_timeout_ms) {
                    Err(_) => {
                        self.info_print("WARNING: Engine Thead Timed Out");
                        return false;
                    }
                    Ok(EngineResponseMessageType::StillCalculatingStatus(false)) => {
                        let _ = cs.send(EngineControlMessageType::GiveMeYourBestMoveSoFar);
                        match rr.recv_timeout(response_timeout_ms) {
                            Ok(EngineResponseMessageType::BestMoveFound(Some(best_move))) => {
                                self.give_response(generate_response(ResponseTokens::BestMove(
                                    best_move.get_long_algebraic(),
                                )));
                                done_status = true;
                            }
                            Ok(_) => { /* other responses ignored */ }
                            Err(_) => {
                                self.info_print("WARNING: Engine Thead Timed Out");
                            }
                        }
                    }
                    Ok(_) => { /* still calculating or other status, continue */ }
                }

                let _ = cs.send(EngineControlMessageType::GiveMeAStringToLog);
                match rr.recv_timeout(response_timeout_ms) {
                    Ok(EngineResponseMessageType::StringToLog(Some(s))) => {
                        self.info_print(&s);
                    }
                    Ok(_) => { /* nothing to log */ }
                    Err(_) => {
                        self.info_print("WARNING: Engine Thead Timed Out");
                    }
                }
            }
        }
        done_status
    }

    /// Stop the calculation
    fn force_stop_calculate(&mut self) {
        if let Some(cs) = &self.command_sender {
            if let Some(flag) = &self.run_engine_thread {
                let _ = cs.send(EngineControlMessageType::StopNow);
                flag.store(false, Ordering::Relaxed);
            }
        }
    }

    /// Used for debugging
    fn info_print(&self, x: &str) {
        self.give_response(generate_response(ResponseTokens::Info(InfoToken::String(
            format!("{x}"),
        ))));
    }

    /// Engine Definitions

    /// This is the best engine
    fn create_best_engine(
        &self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Box<dyn ChessEngineThreadTrait> {
        self.create_engine_2(
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
        )
    }
    /// This is the easiest engine level 1
    fn create_engine_1(
        &self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Box<dyn ChessEngineThreadTrait> {
        Box::new(EngineRandom::new(
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
        ))
    }
    
    /// This is the engine level 2
    fn create_engine_2(
        &self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Box<dyn ChessEngineThreadTrait> {
        Box::new(crate::engine_greedy::EngineGreedy::new(
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
        ))
    }
    /*
    /// This is the engine level 3
    fn create_engine_3(
        &self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Box<dyn ChessEngineThreadTrait> {
        Box::new(EngineMinimax1DeepV0::new(
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
        ))
    }
    /// This is the engine level 4
    fn create_engine_4(
        &self,
        starting_position: GameState,
        calculation_time_s: f32,
        command_receiver: mpsc::Receiver<EngineControlMessageType>,
        response_sender: mpsc::Sender<EngineResponseMessageType>,
    ) -> Box<dyn ChessEngineThreadTrait> {
        Box::new(EngineMinimax1DeepV1::new(
            starting_position,
            calculation_time_s,
            command_receiver,
            response_sender,
        ))
    }       
    */
}
