use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
    time::Duration,
};

use rand::{thread_rng, seq::IteratorRandom};

use crate::{chess_move::ChessMove, game_state::GameState, move_logic::{self, apply_move_to_game, generate_all_moves}};

enum UCISetPositionValueTokens {
    Value((String, String)),
    Clear(String),
    ClearAll,
}

enum OptionTypeToken {
    Check,
    Spin,
    Combo,
    Button,
    String,
}

enum OptionDescriptionToken {
    Default,
    Min,
    Max,
    Var,
}

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

enum RegisterToken {
    Value(String),
}

enum PositionToken {
    Fen(String),
    StartPosition,
}
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
enum CommandTokens {
    Uci,
    Debug(bool),
    IsReady,
    SetOption(OptionToken),
    Register(RegisterToken),
    UciNewGame,
    Position((PositionToken,Vec<String>)),
    Go(GoTokens),
    Stop,
    PonderHit,
    Quit,
}

// Maps an input string to tokens
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
                let mut value = None;
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
                            value = Some(value_parts.join(" "));
                        }
                        _ => {
                            words.next();
                        }
                    }
                }
                // You may want to match name to OptionToken here
                // For now, just push a generic SetOption
                tokens.push(CommandTokens::SetOption(OptionToken::Hash(
                    value.and_then(|v| v.parse().ok()).unwrap_or(0),
                )));
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
                    tokens.push(CommandTokens::Position((position_token,moves)));
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

enum IDTokens {
    Name(String),
    Author(String),
}

enum CopyProtectionToken {
    Ok,
    Error,
}

enum RegistrationToken {
    Ok,
    Error,
    Checking,
}

enum Score {
    CP(f32),
    Mate(u8),
    LowerBound,
    UpperBound,
}

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

// Maps a token to the correct string
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
                format!("option name UCI_Elo type spin default {}", val)
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

#[derive(Clone, Copy, Debug)]
enum UCIstate {
    Startup,
    WaitStartup,
    WaitBootComplete,
    Idle,
    MonitorCalculation,
}

enum OptionName {}

enum OptionValue {}

pub struct UCI {
    uci_state: UCIstate,
    command_rx: Receiver<String>,
    response_tx: Sender<String>,
    position_to_analyze: Option<GameState>,
}

impl UCI {
    pub fn new(command_rx: Receiver<String>, response_tx: Sender<String>) -> Self {
        UCI {
            uci_state: UCIstate::Startup,
            command_rx,
            response_tx,
            position_to_analyze: None,
        }
    }

    // Master state matchine
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
                            CommandTokens::Position((position,moves)) => {
                                self.setup_position(position,moves);
                                self.uci_state
                            }
                            CommandTokens::Go(go) => {
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
                    if self.poll_calculate_check_if_done() {
                        // Done, go back to IDLE
                        UCIstate::Idle
                    } else {
                        // Keep waiting
                        self.uci_state
                    }
                }
            }
        };

        // State updated
        self.uci_state = next_state;
    }

    // Gets a command from the input
    fn get_command(&mut self) -> Option<String> {
        self.command_rx.try_recv().ok()
    }

    // Gives a response to the output
    fn give_response(&mut self, response: String) {
        let _ = self.response_tx.send(response);
    }

    // Use this for launching a thread with boot-up actions
    fn launch_boot_actions(&mut self) {}

    // Check if boot action thread is done
    fn are_boot_actions_done(&mut self) -> bool {
        true
    }

    // Handler for set_options
    fn set_options(&mut self, opt: &OptionToken) {
        // TODO
    }

    // Use this for cleaning up state during a quite
    fn quit_cleanup(&mut self) {}

    // Gives the name and author
    fn give_name_and_author(&mut self) {
        self.give_response(generate_response(ResponseTokens::ID(IDTokens::Name(
            "Plum Chess".into(),
        ))));
        self.give_response(generate_response(ResponseTokens::ID(IDTokens::Author(
            "jwkunz".into(),
        ))));
    }

    // Gives changeable options
    fn give_options_that_can_change(&mut self) {
        // TODO add these changeable options
        self.give_response(generate_response(ResponseTokens::UciOK));
    }

    // Initial response on bootup
    fn respond_to_uci_init(&mut self) {
        self.give_name_and_author();
        self.give_options_that_can_change();
    }

    // Setup a position
    fn setup_position(&mut self,position : &PositionToken ,moves : &Vec<String>) {
        let mut game : GameState;
        match position {
            PositionToken::Fen(x) => {
                if let Ok(g) = GameState::from_fen(x) {
                    game = g;
                } else {
                    self.position_to_analyze = None;
                    return;
                }
            }
            // Parse all the moves
            PositionToken::StartPosition => {
                game = GameState::new_game();
            }
        }
        
        for move_description in moves {
            if let Some(m) = ChessMove::from_long_algebraic(move_description) {
                game = apply_move_to_game(&game, &m);
            } else {
                self.position_to_analyze = None;
                return;
            }
        }  

        self.debug_print(&format!("Searching game:{}",game.get_fen()));
        self.position_to_analyze = Some(game); 
    }

    // GO command
    fn go_launch_calculate(&mut self, go: &GoTokens) {}

    // Polls the calculate and checks if done
    fn poll_calculate_check_if_done(&mut self) -> bool {
        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));

        if let Some(x) = self.position_to_analyze.clone(){
            if let Ok(moves) = generate_all_moves(&x){
                let mut rng = thread_rng();
                if let Some(random_move) = moves.iter().choose(&mut rng) {
                    self.give_response(generate_response(ResponseTokens::BestMove(random_move.description.to_long_algebraic())));
                    return true;
                }
            }
        }
        false
    }

    // Stop the calculation
    fn force_stop_calculate(&mut self) {}

    // Used for debugging
    fn debug_print(&mut self, x : &str){
        self.give_response(generate_response(ResponseTokens::Info(InfoToken::String(format!("DEBUG:{x}")))));
    }
}
