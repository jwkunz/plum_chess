//! Version 7 baseline performance runner.
//!
//! Usage:
//! `cargo run --release --bin v7_perf_baseline`
//! `cargo run --release --bin v7_perf_baseline -- --depth 5`

use plum_chess::move_generation::legal_move_generator::FastLegalMoveGenerator;
use plum_chess::search::board_scoring::{BoardScorer, EndgameTaperedScorerV14};
use plum_chess::search::iterative_deepening_v15::{
    iterative_deepening_search_with_tt, SearchConfig,
};
use plum_chess::search::transposition_table_v11::TranspositionTable;
use plum_chess::utils::fen_parser::parse_fen;

fn parse_arg_u8(flag: &str, default: u8) -> u8 {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            if let Ok(v) = args[i + 1].parse::<u8>() {
                return v.max(1);
            }
        }
    }
    default
}

fn run_case(
    name: &str,
    fen: &str,
    depth: u8,
    scorer: &impl BoardScorer,
    generator: &FastLegalMoveGenerator,
) -> Result<(), String> {
    let game = parse_fen(fen)?;
    let mut tt = TranspositionTable::new_with_mb(64);
    let result = iterative_deepening_search_with_tt(
        &game,
        generator,
        scorer,
        SearchConfig {
            max_depth: depth,
            ..SearchConfig::default()
        },
        &mut tt,
    )
    .map_err(|e| format!("{e:?}"))?;
    println!(
        "{name}: depth={} nodes={} elapsed_ms={} nps={} best_score={} best_move={:?}",
        result.reached_depth,
        result.nodes,
        result.elapsed_ms,
        result.nps,
        result.best_score,
        result.best_move
    );
    Ok(())
}

fn main() -> Result<(), String> {
    let depth = parse_arg_u8("--depth", 4);
    let scorer = EndgameTaperedScorerV14::alpha_zero();
    let generator = FastLegalMoveGenerator;
    let suite = [
        (
            "startpos",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ),
        (
            "classical_mid",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        ),
        (
            "tactical",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        ),
        ("end_kpk", "8/8/8/8/8/4k3/4P3/4K3 w - - 0 1"),
    ];
    println!("v7 baseline run: depth={depth}");
    for (name, fen) in suite {
        run_case(name, fen, depth, &scorer, &generator)?;
    }
    Ok(())
}

