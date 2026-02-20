use plum_chess::engines::engine_iterative_v16::IterativeEngine;
use plum_chess::engines::engine_trait::{Engine, GoParams};
use plum_chess::game_state::game_state::GameState;
use plum_chess::utils::fen_parser::parse_fen;
use std::env;
use std::time::Instant;

fn parse_arg<T: std::str::FromStr>(args: &[String], idx: usize, default: T) -> T {
    args.get(idx)
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let max_threads: usize = parse_arg(&args, 1, 8usize);
    let depth: u8 = parse_arg(&args, 2, 4u8);
    let runs_per_thread: usize = parse_arg(&args, 3, 3usize);

    let positions = vec![
        GameState::new_game(),
        parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap_or_else(|_| GameState::new_game()),
        parse_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10")
            .unwrap_or_else(|_| GameState::new_game()),
    ];

    println!(
        "Thread scaling benchmark: max_threads={} depth={} runs_per_thread={}",
        max_threads, depth, runs_per_thread
    );
    println!("threads,total_ms,avg_ms,positions,searched_runs");

    for threads in 1..=max_threads.max(1) {
        let mut total_ms: u128 = 0;
        let mut searched_runs = 0usize;

        for _ in 0..runs_per_thread.max(1) {
            for game in &positions {
                let mut engine = IterativeEngine::new(depth);
                let _ = engine.set_option("OwnBook", "false");
                let _ = engine.set_option("ThreadingModel", "LazySmp");
                let _ = engine.set_option("Threads", &threads.to_string());
                let _ = engine.set_option("RootParallelMinDepth", "1");
                let _ = engine.set_option("RootParallelMinMoves", "2");

                let start = Instant::now();
                let _ = engine.choose_move(
                    game,
                    &GoParams {
                        depth: Some(depth),
                        ..GoParams::default()
                    },
                );
                total_ms += start.elapsed().as_millis();
                searched_runs += 1;
            }
        }

        let avg_ms = if searched_runs == 0 {
            0.0
        } else {
            total_ms as f64 / searched_runs as f64
        };
        println!(
            "{},{},{:.2},{},{}",
            threads,
            total_ms,
            avg_ms,
            positions.len(),
            searched_runs
        );
    }
}
