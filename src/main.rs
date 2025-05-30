use std::io::{self, BufRead, Write};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

mod uci;
use uci::UCI;

fn main() {
    let (command_tx, command_rx) = channel::<String>();
    let (response_tx, response_rx) = channel::<String>();

    // Spawn UCI thread
    thread::spawn(move || {
        let mut uci = UCI::new(command_rx, response_tx);
        loop {
            uci.tick();
            // Sleep briefly to avoid busy-waiting
            thread::sleep(Duration::from_millis(10));
        }
    });

    thread::spawn(move || loop {
        while let Ok(response) = response_rx.try_recv() {
            println!("{}", response);
            io::stdout().flush().ok();
        }
        thread::sleep(Duration::from_millis(10));
    });

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut input = String::new();

    loop {
        // Check for input from stdin (non-blocking)
        input.clear();
        // Use non-blocking read by checking if stdin has data available
        if let Ok(n) = stdin_lock.read_line(&mut input) {
            if n > 0 {
                let trimmed = input.trim_end().to_string();
                if !trimmed.is_empty() {
                    let _ = command_tx.send(trimmed);
                }
            }
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }
}
