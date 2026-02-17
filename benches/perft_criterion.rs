use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use plum_chess::game_state::game_state::GameState;
use plum_chess::move_generation::perft::perft_legal;

#[derive(Clone, Copy)]
struct BenchCase {
    name: &'static str,
    fen: &'static str,
    expected_nodes: &'static [u64],
}

const STARTPOS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

const CASES_QUICK: &[BenchCase] = &[
    BenchCase {
        name: "position_1",
        fen: STARTPOS_FEN,
        expected_nodes: &[20, 400, 8902],
    },
    BenchCase {
        name: "position_2",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0",
        expected_nodes: &[48, 2039],
    },
    BenchCase {
        name: "position_3",
        fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        expected_nodes: &[14, 191, 2812],
    },
];

const CASES_STANDARD: &[BenchCase] = &[
    BenchCase {
        name: "position_1",
        fen: STARTPOS_FEN,
        expected_nodes: &[20, 400, 8902, 197_281],
    },
    BenchCase {
        name: "position_2",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0",
        expected_nodes: &[48, 2039, 97_862],
    },
    BenchCase {
        name: "position_3",
        fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        expected_nodes: &[14, 191, 2812, 43_238, 674_624],
    },
    BenchCase {
        name: "position_4",
        fen: "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1",
        expected_nodes: &[6, 264, 9467, 422_333],
    },
    BenchCase {
        name: "position_5",
        fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        expected_nodes: &[44, 1486, 62_379, 2_103_487],
    },
    BenchCase {
        name: "position_6",
        fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        expected_nodes: &[46, 2079, 89_890, 3_894_594],
    },
];

fn selected_cases() -> &'static [BenchCase] {
    match std::env::var("PLUM_BENCH_SUITE") {
        Ok(value) if value.eq_ignore_ascii_case("standard") => CASES_STANDARD,
        _ => CASES_QUICK,
    }
}

fn bench_perft(c: &mut Criterion) {
    let suite_name = match std::env::var("PLUM_BENCH_SUITE") {
        Ok(value) if value.eq_ignore_ascii_case("standard") => "standard",
        _ => "quick",
    };

    let mut group = c.benchmark_group(format!("perft_{suite_name}"));
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(4));
    group.sample_size(20);

    for case in selected_cases() {
        let game = GameState::from_fen(case.fen).expect("benchmark FEN should parse");

        for (depth_idx, expected_nodes) in case.expected_nodes.iter().enumerate() {
            let depth = (depth_idx + 1) as u8;

            // Correctness guard before benchmarking.
            let warmup = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(
                warmup.nodes as u64, *expected_nodes,
                "node mismatch in warmup for {} depth {}",
                case.name, depth
            );

            group.throughput(Throughput::Elements(*expected_nodes));
            let bench_name = format!("{}_d{}", case.name, depth);
            let bench_game = game.clone();

            group.bench_with_input(
                BenchmarkId::from_parameter(bench_name),
                expected_nodes,
                |b, expected| {
                    b.iter(|| {
                        let count = perft_legal(black_box(&bench_game), black_box(depth))
                            .expect("perft benchmark run should succeed");
                        assert_eq!(count.nodes as u64, *expected);
                        black_box(count.nodes)
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(perft_benches, bench_perft);
criterion_main!(perft_benches);
