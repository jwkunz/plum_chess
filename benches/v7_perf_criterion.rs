use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use plum_chess::move_generation::legal_move_generator::FastLegalMoveGenerator;
use plum_chess::search::board_scoring::EndgameTaperedScorerV14;
use plum_chess::search::iterative_deepening_v15::{
    iterative_deepening_search_with_tt, SearchConfig,
};
use plum_chess::search::transposition_table_v11::TranspositionTable;
use plum_chess::utils::fen_parser::parse_fen;

#[derive(Clone, Copy)]
struct PerfCase {
    name: &'static str,
    fen: &'static str,
}

const CASES: &[PerfCase] = &[
    PerfCase {
        name: "startpos",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    },
    PerfCase {
        name: "classical_mid",
        fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    },
    PerfCase {
        name: "tactical",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    },
    PerfCase {
        name: "end_kpk",
        fen: "8/8/8/8/8/4k3/4P3/4K3 w - - 0 1",
    },
];

fn bench_v7_search_perf(c: &mut Criterion) {
    let depth = std::env::var("PLUM_V7_DEPTH")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(4)
        .max(1);

    let mut group = c.benchmark_group("v7_search_perf");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(4));
    group.sample_size(20);

    let scorer = EndgameTaperedScorerV14::alpha_zero();
    let generator = FastLegalMoveGenerator;

    for case in CASES {
        let game = parse_fen(case.fen).expect("benchmark FEN should parse");
        group.bench_with_input(
            BenchmarkId::new(case.name, format!("d{depth}")),
            &game,
            |b, game| {
                b.iter(|| {
                    let mut tt = TranspositionTable::new_with_mb(64);
                    let result = iterative_deepening_search_with_tt(
                        black_box(game),
                        black_box(&generator),
                        black_box(&scorer),
                        black_box(SearchConfig {
                            max_depth: depth,
                            ..SearchConfig::default()
                        }),
                        black_box(&mut tt),
                    )
                    .expect("search should run");
                    black_box(result.nodes)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(v7_perf_benches, bench_v7_search_perf);
criterion_main!(v7_perf_benches);
