//! Standalone engine-vs-engine series runner.
//!
//! Run with:
//! `cargo run --release --bin engine_match_series`
//! `cargo run --release --bin engine_match_series -- --verbose`

use plum_chess::engines::engine_trait::{Engine, EngineOutput, GoParams};
use plum_chess::game_state::game_state::GameState;
use plum_chess::move_generation::legal_move_generator::FastLegalMoveGenerator;
use plum_chess::search::board_scoring::{
    AlphaZeroMetric, AlphaZeroPlusLegalMoves, BoardScorer,
};
use plum_chess::search::iterative_deepening::{iterative_deepening_search, SearchConfig};
use plum_chess::utils::engine_match_harness::{
    play_engine_match_series, MatchConfig, MatchSeriesConfig,
};

struct ConfigurableIterativeEngine<S: BoardScorer + Send + Sync + 'static> {
    scorer: S,
    default_depth: u8,
    generator: FastLegalMoveGenerator,
}

impl<S: BoardScorer + Send + Sync + 'static> ConfigurableIterativeEngine<S> {
    fn new(scorer: S, default_depth: u8) -> Self {
        Self {
            scorer,
            default_depth,
            generator: FastLegalMoveGenerator,
        }
    }
}

impl<S: BoardScorer + Send + Sync + 'static> Engine for ConfigurableIterativeEngine<S> {
    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        let depth = params.depth.unwrap_or(self.default_depth).max(1);
        let result = iterative_deepening_search(
            game_state,
            &self.generator,
            &self.scorer,
            SearchConfig {
                max_depth: depth,
                movetime_ms: params.movetime_ms,
            },
        )
        .map_err(|e| e.to_string())?;

        Ok(EngineOutput {
            best_move: result.best_move,
            info_lines: Vec::new(),
        })
    }
}

// Use 'cargo run --release --bin engine_match_series -- --verbose' to run this
fn main() -> Result<(), String> {
    let verbose = std::env::args().any(|a| a == "--verbose" || a == "-v");

    // Customize these two lines to experiment with different engines/scorers/depths.
    let player1 =
        || Box::new(ConfigurableIterativeEngine::new(AlphaZeroMetric, 3)) as Box<dyn Engine>;
    let player2 = || {
        Box::new(ConfigurableIterativeEngine::new(AlphaZeroPlusLegalMoves, 3)) as Box<dyn Engine>
    };

    let stats = play_engine_match_series(
        player1,
        player2,
        MatchSeriesConfig {
            games: 100,
            base_seed: 1234,
            per_game: MatchConfig {
                max_plies: 120,
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
Notes:

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

*/
