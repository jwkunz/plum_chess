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
        Box::new(plum_chess::engines::engine_iterative_v8::IterativeEngine::new_alpha_zero(6))
            as Box<dyn Engine>
    };
    let player2 = || {
        Box::new(plum_chess::engines::engine_iterative_v12::IterativeEngine::new_alpha_zero(6))
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

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v3::IterativeEngine::new_alpha_zero(5) = 5 wins @ 175.9 ms per move
vs
plum_chess::engines::engine_iterative_v4::IterativeEngine::new_alpha_zero(5) = 0 wins @ 31.1 ms per move
Conclusion:  The killer move and history table made the search go faster, but somehow caused more loss.  3 is a stronger engine.

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v3::IterativeEngine::new_alpha_zero(4) = 1 wins @ 54.187 ms per move
vs
plum_chess::engines::engine_iterative_v5::IterativeEngine::new_alpha_zero(4) = 2 wins @ 10.725 ms per move
Conclusion:  Adding the aspiration windows and null move pruning for significant speed up without loss of performance

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v5::IterativeEngine::new_alpha_zero(5) = 1 wins @ 18.534 ms per move
vs
plum_chess::engines::engine_iterative_v6::IterativeEngine::new_alpha_zero(5) = 1 wins @ 17.858 ms per move
Conclusion:  Adding PVS shaved off about a ms per move.

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v6::IterativeEngine::new_alpha_zero(5) = 1 wins @ 11.514 ms per move
vs
plum_chess::engines::engine_iterative_v7::IterativeEngine::new_alpha_zero(5) = 3 wins @ 12.534 ms per move
Conclusion:  Adding countermove + continuation-history move ordering made slight stronger with a bit of a slow down

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v7::IterativeEngine::new_alpha_zero(5) = 3 wins @ 17.672 ms per move
vs
plum_chess::engines::engine_iterative_v8::IterativeEngine::new_alpha_zero(5) = 2 wins @ 14.290 ms per move
Conclusion:  Adding fast SEE-style estimate from moved/captured/promotion piece values seemed to speed things up

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v8::IterativeEngine::new_alpha_zero(6) = 7 wins @ 38.336 ms per move
vs
plum_chess::engines::engine_iterative_v9::IterativeEngine::new_alpha_zero(6) = 2 wins @ 18.498 ms per move
Conclusion:  The two improvements in v9 made it faster, but weaker

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v8::IterativeEngine::new_alpha_zero(6) = 6 wins @ 38.336 ms per move
vs
plum_chess::engines::engine_iterative_v10::IterativeEngine::new_alpha_zero(6) = 2 wins @ 21.574 ms per move
Conclusion:  Null move verification search is faster, but still weaker

---

In a 10 game series of 200 plies:
plum_chess::engines::engine_iterative_v8::IterativeEngine::new_alpha_zero(6) = 7 wins @ 42.836 ms per move
vs
plum_chess::engines::engine_iterative_v11::IterativeEngine::new_alpha_zero(6) = 0 wins @ 19.491 ms per move
Conclusion:  Null move verification search is faster, but still weaker

*/
