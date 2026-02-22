use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use plum_chess::engines::engine_iterative_v16::IterativeEngine as IterativeEngineV16;
use plum_chess::engines::engine_iterative_v17::IterativeEngineV17;
use plum_chess::engines::engine_trait::{Engine, GoParams};
use plum_chess::game_state::game_state::GameState;
use plum_chess::utils::engine_match_harness::{
    play_engine_match_from_state, play_engine_match_series, MatchConfig, MatchSeriesConfig,
};

fn v16_factory(depth: u8) -> Box<dyn Engine> {
    Box::new(IterativeEngineV16::new_alpha_zero(depth))
}

fn v17_factory(depth: u8) -> Box<dyn Engine> {
    Box::new(IterativeEngineV17::new_alpha_zero(depth))
}

fn bench_v6_opening_series(c: &mut Criterion) {
    let depth = std::env::var("PLUM_V6_DEPTH")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(4)
        .max(1);
    let games = std::env::var("PLUM_V6_GAMES")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(2)
        .max(1);

    let mut group = c.benchmark_group("v6_acceptance_opening_series");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(10);

    group.bench_function("v16_vs_v17", |b| {
        b.iter(|| {
            let stats = play_engine_match_series(
                || v16_factory(depth),
                || v17_factory(depth),
                MatchSeriesConfig {
                    games,
                    base_seed: 4242,
                    per_game: MatchConfig {
                        max_plies: 120,
                        opening_min_plies: 2,
                        opening_max_plies: 4,
                        go_params: GoParams {
                            depth: Some(depth),
                            movetime_ms: Some(30),
                            ..GoParams::default()
                        },
                    },
                    verbose: false,
                },
            )
            .expect("series should run");
            black_box(stats.games)
        });
    });

    group.finish();
}

fn bench_v6_endgame_suite(c: &mut Criterion) {
    let depth = std::env::var("PLUM_V6_DEPTH")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(4)
        .max(1);
    let fens = [
        "8/8/8/8/8/4k3/4P3/4K3 w - - 0 1",
        "k7/7P/8/8/8/8/8/K7 w - - 0 1",
        "8/8/8/8/8/8/4KB2/6Nk w - - 0 1",
    ];
    let mut starts = Vec::with_capacity(fens.len());
    for fen in fens {
        starts.push(GameState::from_fen(fen).expect("FEN should parse"));
    }

    let config = MatchConfig {
        max_plies: 80,
        opening_min_plies: 0,
        opening_max_plies: 0,
        go_params: GoParams {
            depth: Some(depth),
            movetime_ms: Some(30),
            ..GoParams::default()
        },
    };

    let mut group = c.benchmark_group("v6_acceptance_endgame_suite");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(10);

    group.bench_function("v16_v17_curated", |b| {
        b.iter(|| {
            let mut completed = 0u32;
            for (idx, start) in starts.iter().enumerate() {
                let _ = play_engine_match_from_state(
                    v16_factory(depth),
                    v17_factory(depth),
                    start.clone(),
                    10_000 + idx as u64,
                    config.clone(),
                )
                .expect("match should run");
                completed += 1;
            }
            black_box(completed)
        });
    });
    group.finish();
}

criterion_group!(
    v6_acceptance_benches,
    bench_v6_opening_series,
    bench_v6_endgame_suite
);
criterion_main!(v6_acceptance_benches);

