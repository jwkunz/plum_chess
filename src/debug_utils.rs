use std::fs::read;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Runs Stockfish perft for a given FEN and depth, returning the node count.
///
/// # Arguments
///
/// * `fen` - The FEN string describing the chess position.
/// * `depth` - The perft search depth.
///
/// # Returns
///
/// A `usize` representing the node count reported by Stockfish.
///
/// # Example
///
/// ```
/// let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
/// let nodes = run_stockfish_perft(fen, 3)?;
/// println!("Nodes: {}", nodes);
/// ```
pub fn run_stockfish_perft(fen: &str, depth: u32) -> std::io::Result<(usize,Vec<String>)> {
    // Spawn Stockfish
    let mut child = Command::new("stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialization commands
    writeln!(stdin, "uci")?;
    stdin.flush()?;
    thread::sleep(Duration::from_millis(100));

    writeln!(stdin, "position fen {}", fen)?;
    stdin.flush()?;
    thread::sleep(Duration::from_millis(100));

    // Drain options
    let mut buf = String::new();
    while reader.read_line(&mut buf)? > 0 {
        if buf == "uciok\n"{
            break;
        }
        buf.clear(); // just discard the content
    }

    writeln!(stdin, "go perft {}", depth)?;
    stdin.flush()?;

    // Collect perft lines
    let mut result_lines = Vec::new();
    let mut line = String::new();
    let mut last_number : usize = 0;

    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("Nodes searched:") {
            if let Some(num_str) = line[pos + 15..].trim().split_whitespace().next() {
                if let Ok(num) = num_str.parse::<usize>() {
                    last_number = num;
                    break;
                }
            }
        } else if !trimmed.is_empty() {
            // perft lines look like: "a2a3: 380"
            result_lines.push(trimmed.to_string());
        }
        line.clear();
    }

    // Wait for process termination
    writeln!(stdin, "quit")?;
    let _ = child.wait();

    result_lines.sort();

    Ok((last_number,result_lines))
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test_run_stockfish_perft(){
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let (count,nodes) = run_stockfish_perft(fen, 3).unwrap();
        assert_eq!(count,8902);
    }
}