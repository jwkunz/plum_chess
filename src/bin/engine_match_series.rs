//! Standalone engine-vs-engine series runner.
//!
//! Run with:
//! `cargo run --release --bin engine_match_series`

use plum_chess::engines::engine_trait::{Engine, EngineOutput, GoParams};
use plum_chess::game_state::game_state::GameState;
use plum_chess::move_generation::legal_move_generator::FastLegalMoveGenerator;
use plum_chess::search::board_scoring::{
    AlphaZeroMetric, AlphaZeroPlusLegalMoves, BoardScorer, MaterialScorer,
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

// Use 'cargo run --bin engine_match_series' to run this
fn main() -> Result<(), String> {
    // Customize these two lines to experiment with different engines/scorers/depths.
    let player1 =
        || Box::new(ConfigurableIterativeEngine::new(MaterialScorer, 1)) as Box<dyn Engine>;
    let player2 = || {
        Box::new(ConfigurableIterativeEngine::new(AlphaZeroPlusLegalMoves, 2)) as Box<dyn Engine>
    };

    // Keep this around so you can quickly switch scorer experiments in code.
    let _alternate =
        || Box::new(ConfigurableIterativeEngine::new(AlphaZeroMetric, 2)) as Box<dyn Engine>;

    let stats = play_engine_match_series(
        player1,
        player2,
        MatchSeriesConfig {
            games: 9,
            base_seed: 1234,
            per_game: MatchConfig {
                max_plies: 60,
                opening_min_plies: 2,
                opening_max_plies: 6,
                ..MatchConfig::default()
            },
        },
    )?;

    println!("{}", stats.report());
    println!("outcomes: {:?}", stats.outcomes);
    Ok(())
}
