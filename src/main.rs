//! Binary entry point for the UCI chess engine process.
//!
//! The executable delegates runtime behavior to the UCI subsystem, which
//! manages command parsing, engine selection, and move responses over stdio.

fn main() {
    if let Err(err) = plum_chess::uci::uci_top::run_stdio_loop() {
        eprintln!("uci loop error: {}", err);
    }
}
