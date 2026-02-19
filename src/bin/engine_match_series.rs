//! Standalone engine-vs-engine series runner.
//!
//! Run with:
//! `cargo run --release --bin engine_match_series`
//! `cargo run --release --bin engine_match_series -- --verbose`

use plum_chess::engines::engine_trait::Engine;
use plum_chess::utils::engine_match_harness::{
    play_engine_match_series, MatchConfig, MatchSeriesConfig,
};

// Use 'cargo run --release --bin engine_match_series -- --verbose' to run this
fn main() -> Result<(), String> {
    let verbose = std::env::args().any(|a| a == "--verbose" || a == "-v");

    // Customize these two lines to experiment with different engines/scorers/depths.
    let player1 = || {
        Box::new(plum_chess::engines::engine_iterative_v1::IterativeEngine::new_alpha_zero(5))
            as Box<dyn Engine>
    };
    let player2 = || {
        Box::new(plum_chess::engines::engine_iterative_v2::IterativeEngine::new_alpha_zero(5))
            as Box<dyn Engine>
    };

    let stats = play_engine_match_series(
        player1,
        player2,
        MatchSeriesConfig {
            games: 10,
            base_seed: 1234,
            per_game: MatchConfig {
                max_plies: 200,
                opening_min_plies: 2,
                opening_max_plies: 6,
                ..MatchConfig::default()
            },
            verbose,
        },
    )?;

    println!("{}", stats.report());
    println!("outcomes: {:?}", stats.outcomes);
    Ok(())
}

/*
Tuning Notes:

In a 100 game series of 120 plies:
ConfigurableIterativeEngine::new(AlphaZeroPlusLegalMoves, 3) = 20 wins @ 18.1 ms per move
vs
ConfigurableIterativeEngine::new(MaterialScorer, 3) = 9 wins @ 6.3 ms per move
Conclusion:  Alpha Zero weightings are superior in short decisions games

---

In a 100 game series of 120 plies:
ConfigurableIterativeEngine::new(AlphaZeroPlusLegalMoves, 3) = 22 wins @ 5.938 ms per move
vs
ConfigurableIterativeEngine::new(AlphaZeroMetric, 3) = 23 wins @ 18.710 ms per move
Conclusion:  Adding legal moves to Alpha Zero weightings onyl wastes time

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v1::IterativeEngine::new_alpha_zero(5) = 0 wins @ 4.576 ms per move
vs
plum_chess::engines::engine_iterative_v2::IterativeEngine::new_alpha_zero(5) = 0 wins @ 12.605 ms per move
Conclusion:  Adding endgame hueristics helps

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v1::IterativeEngine::new_alpha_zero(5) = 0 wins @ 81.907 ms per move
vs
plum_chess::engines::engine_iterative_v3::IterativeEngine::new_alpha_zero(5) = 0 wins @ 88.626 ms per move
Conclusion:  Adding promotion hueristics does little

*/
