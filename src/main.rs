use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use chrono::Local;


use plum_chess::uci_interface::UCI;

/// The main entry point for the Plum Chess engine.
/// 
/// This function sets up logging, spawns the UCI protocol handler thread, and manages
/// communication between the engine and the user (or GUI) via standard input and output.
/// 
/// It logs all received commands and engine responses with timestamps to a log file,
/// and ensures that the engine runs responsively by using non-blocking I/O and periodic sleeps.
fn main() {
    // Name of the log file to which all engine activity will be written.
    let log_file_name = "Plum_Chess_log.txt";
    // Timestamp format including milliseconds.
    let time_format = "%Y-%m-%d %H:%M:%S%.3f";

    // Initialize the log file: clear it if it exists and write the starting line with a timestamp.
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)      // open for writing
        .truncate(true)   // clear the file if it exists
        .open(log_file_name)
    {
        let timestamp = Local::now().format(time_format);
        let _ = writeln!(file, "[{}] Starting Log", timestamp);
    }

    // Create channels for sending commands to and receiving responses from the UCI thread.
    let (command_tx, command_rx) = channel::<String>();
    let (response_tx, response_rx) = channel::<String>();

    // Spawn the UCI protocol handler thread.
    // This thread runs the UCI state machine, processing commands and generating responses.
    thread::spawn(move || {
        let mut uci = UCI::new(command_rx, response_tx);
        loop {
            uci.tick();
            // Sleep briefly to avoid busy-waiting
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Spawn a thread to handle responses from the engine.
    // This thread logs each response with a timestamp and prints it to stdout.
    thread::spawn(move || loop {
        while let Ok(response) = response_rx.try_recv() {
            // Append the engine's response to the log file with a timestamp.
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file_name) {
                let timestamp = Local::now().format(time_format);
                let _ = writeln!(file, "[{}] Engine Said:{}", timestamp, response);
            }
            // Print the response to stdout.
            println!("{}", response);
            io::stdout().flush().ok();
        }
        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    });

    // Prepare to read commands from stdin.
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut input = String::new();

    // Main loop: read input from stdin, log it, and send it to the UCI thread.
    loop {
        // Clear the input buffer.
        input.clear();
        // Read a line from stdin (blocking until input is available).
        if let Ok(n) = stdin_lock.read_line(&mut input) {
            if n > 0 {
                let trimmed = input.trim_end().to_string();
                if !trimmed.is_empty() {
                    // Append the received command to the log file with a timestamp.
                    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file_name) {
                        let timestamp = Local::now().format(time_format);
                        let _ = writeln!(file, "[{}] Engine Received:{}", timestamp, trimmed);
                    }
                    // Send the command to the UCI thread for processing.
                    let _ = command_tx.send(trimmed);
                }
            }
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }
}
