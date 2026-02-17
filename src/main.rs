fn main() {
    if let Err(err) = plum_chess::uci::uci_top::run_stdio_loop() {
        eprintln!("uci loop error: {}", err);
    }
}
