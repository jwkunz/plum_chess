//! Version 6 acceptance runner.
//!
//! Runs:
//! - V16 vs V17 head-to-head from standard starting positions.
//! - V16 vs V17 on curated endgame start positions.
//!
//! Usage:
//! `cargo run --release --bin v6_acceptance`

use plum_chess::engines::engine_iterative_v16::IterativeEngine as IterativeEngineV16;
use plum_chess::engines::engine_iterative_v17::IterativeEngineV17;
use plum_chess::engines::engine_trait::Engine;
use plum_chess::game_state::game_state::GameState;
use plum_chess::utils::engine_match_harness::{
    play_engine_match_from_state, play_engine_match_series, MatchConfig, MatchOutcome,
    MatchSeriesConfig, MatchSeriesStats,
};

#[derive(Debug, Default)]
struct EndgameSuiteStats {
    games: u32,
    v16_wins: u32,
    v17_wins: u32,
    draws: u32,
    v16_time_ns: u128,
    v17_time_ns: u128,
    v16_moves: u32,
    v17_moves: u32,
}

impl EndgameSuiteStats {
    fn avg_ms_v16(&self) -> f64 {
        if self.v16_moves == 0 {
            0.0
        } else {
            self.v16_time_ns as f64 / self.v16_moves as f64 / 1_000_000.0
        }
    }

    fn avg_ms_v17(&self) -> f64 {
        if self.v17_moves == 0 {
            0.0
        } else {
            self.v17_time_ns as f64 / self.v17_moves as f64 / 1_000_000.0
        }
    }
}

fn v16_factory(depth: u8) -> Box<dyn Engine> {
    Box::new(IterativeEngineV16::new_alpha_zero(depth))
}

fn v17_factory(depth: u8) -> Box<dyn Engine> {
    Box::new(IterativeEngineV17::new_alpha_zero(depth))
}

fn print_series_report(title: &str, stats: &MatchSeriesStats) {
    println!("\n== {} ==", title);
    println!("{}", stats.report());
    let regression_pct = if stats.player1_avg_move_time_ms > 0.0 {
        ((stats.player2_avg_move_time_ms - stats.player1_avg_move_time_ms)
            / stats.player1_avg_move_time_ms)
            * 100.0
    } else {
        0.0
    };
    println!(
        "speed proxy: v16_avg_ms={:.3} v17_avg_ms={:.3} regression={:.2}%",
        stats.player1_avg_move_time_ms, stats.player2_avg_move_time_ms, regression_pct
    );
}

fn run_endgame_suite(depth: u8) -> Result<EndgameSuiteStats, String> {
    let suite = [
        // KPK win chances
        "8/8/8/8/8/4k3/4P3/4K3 w - - 0 1",
        "k7/7P/8/8/8/8/8/K7 w - - 0 1",
        // KBNK conversion
        "8/8/8/8/8/8/4KB2/6Nk w - - 0 1",
        // KRK and KQK technical conversion starts
        "8/8/8/8/8/8/5k2/6KR w - - 0 1",
        "8/8/8/8/8/8/5k2/6QK w - - 0 1",
    ];
    let mut stats = EndgameSuiteStats::default();
    let config = MatchConfig {
        max_plies: 80,
        opening_min_plies: 0,
        opening_max_plies: 0,
        go_params: plum_chess::engines::engine_trait::GoParams {
            depth: Some(depth),
            movetime_ms: Some(40),
            ..Default::default()
        },
    };

    for (idx, fen) in suite.iter().enumerate() {
        let start = GameState::from_fen(fen).map_err(|e| format!("bad suite fen {fen}: {e}"))?;
        for swap in [false, true] {
            let seed = 10_000 + (idx as u64) * 31 + if swap { 1 } else { 0 };
            let result = if !swap {
                play_engine_match_from_state(
                    v16_factory(depth),
                    v17_factory(depth),
                    start.clone(),
                    seed,
                    config.clone(),
                )?
            } else {
                play_engine_match_from_state(
                    v17_factory(depth),
                    v16_factory(depth),
                    start.clone(),
                    seed,
                    config.clone(),
                )?
            };
            stats.games += 1;

            // Attribute move-time usage back to engine identity regardless of color swap.
            if !swap {
                stats.v16_time_ns += result.white_total_time_ns;
                stats.v17_time_ns += result.black_total_time_ns;
                stats.v16_moves += result.white_move_count;
                stats.v17_moves += result.black_move_count;
            } else {
                stats.v17_time_ns += result.white_total_time_ns;
                stats.v16_time_ns += result.black_total_time_ns;
                stats.v17_moves += result.white_move_count;
                stats.v16_moves += result.black_move_count;
            }

            match result.outcome {
                MatchOutcome::WhiteWinCheckmate => {
                    if !swap {
                        stats.v16_wins += 1;
                    } else {
                        stats.v17_wins += 1;
                    }
                }
                MatchOutcome::BlackWinCheckmate => {
                    if !swap {
                        stats.v17_wins += 1;
                    } else {
                        stats.v16_wins += 1;
                    }
                }
                MatchOutcome::DrawStalemate
                | MatchOutcome::DrawRepetition
                | MatchOutcome::DrawFiftyMoveRule
                | MatchOutcome::DrawMaxPlies => {
                    stats.draws += 1;
                }
            }
        }
    }

    Ok(stats)
}

fn main() -> Result<(), String> {
    let mut depth = 4u8;
    let mut games = 6u16;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--depth" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u8>() {
                    depth = v.max(1);
                }
                i += 1;
            }
            "--games" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u16>() {
                    games = v.max(2);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let opening_series = play_engine_match_series(
        || v16_factory(depth),
        || v17_factory(depth),
        MatchSeriesConfig {
            games,
            base_seed: 4242,
            per_game: MatchConfig {
                max_plies: 200,
                opening_min_plies: 2,
                opening_max_plies: 6,
                go_params: plum_chess::engines::engine_trait::GoParams {
                    depth: Some(depth),
                    movetime_ms: Some(40),
                    ..Default::default()
                },
            },
            verbose: false,
        },
    )?;
    print_series_report("Opening/Middlegame Series (V16=Player1, V17=Player2)", &opening_series);

    let endgame = run_endgame_suite(depth)?;
    let endgame_regression = if endgame.avg_ms_v16() > 0.0 {
        ((endgame.avg_ms_v17() - endgame.avg_ms_v16()) / endgame.avg_ms_v16()) * 100.0
    } else {
        0.0
    };
    println!("\n== Endgame Suite ==");
    println!(
        "games={} v16_wins={} v17_wins={} draws={}",
        endgame.games, endgame.v16_wins, endgame.v17_wins, endgame.draws
    );
    println!(
        "speed proxy: v16_avg_ms={:.3} v17_avg_ms={:.3} regression={:.2}%",
        endgame.avg_ms_v16(),
        endgame.avg_ms_v17(),
        endgame_regression
    );

    Ok(())
}
