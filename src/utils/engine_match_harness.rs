//! Minimal head-to-head engine match harness for local testing.
//!
//! This module runs two `Engine` implementations against each other without
//! UCI I/O, with an optional seeded random opening prefix.

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Instant;

use crate::engines::engine_trait::{Engine, GoParams};
use crate::game_state::chess_types::Color;
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::tables::opening_book::OpeningBook;
use crate::utils::long_algebraic::move_description_to_long_algebraic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    WhiteWinCheckmate,
    BlackWinCheckmate,
    DrawStalemate,
    DrawRepetition,
    DrawFiftyMoveRule,
    DrawMaxPlies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerId {
    Player1,
    Player2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesOutcome {
    PlayerWinCheckmate { player: PlayerId, color: Color },
    DrawStalemate,
    DrawRepetition,
    DrawFiftyMoveRule,
    DrawMaxPlies,
}

#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub max_plies: u16,
    pub opening_min_plies: u8,
    pub opening_max_plies: u8,
    pub go_params: GoParams,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            max_plies: 300,
            opening_min_plies: 2,
            opening_max_plies: 8,
            go_params: GoParams::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub outcome: MatchOutcome,
    pub final_state: GameState,
    pub opening_moves_lan: Vec<String>,
    pub played_moves_lan: Vec<String>,
    pub white_move_count: u32,
    pub black_move_count: u32,
    pub white_total_time_ns: u128,
    pub black_total_time_ns: u128,
}

#[derive(Debug, Clone)]
pub struct MatchSeriesConfig {
    pub games: u16,
    pub base_seed: u64,
    pub per_game: MatchConfig,
    pub verbose: bool,
}

impl Default for MatchSeriesConfig {
    fn default() -> Self {
        Self {
            games: 9,
            base_seed: 0,
            per_game: MatchConfig::default(),
            verbose: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MatchSeriesStats {
    pub games: u16,
    pub player1_wins: u16,
    pub player2_wins: u16,
    pub draws: u16,
    pub outcomes: Vec<SeriesOutcome>,
    pub player1_moves: u32,
    pub player2_moves: u32,
    pub player1_total_time_ns: u128,
    pub player2_total_time_ns: u128,
    pub player1_avg_move_time_ms: f64,
    pub player2_avg_move_time_ms: f64,
    pub overall_avg_move_time_ms: f64,
}

impl MatchSeriesStats {
    pub fn report(&self) -> String {
        format!(
            "games={} player1_wins={} player2_wins={} draws={} p1_avg_ms={:.3} p2_avg_ms={:.3} overall_avg_ms={:.3}",
            self.games,
            self.player1_wins,
            self.player2_wins,
            self.draws,
            self.player1_avg_move_time_ms,
            self.player2_avg_move_time_ms,
            self.overall_avg_move_time_ms
        )
    }
}

/// Play a single seeded engine-vs-engine match.
///
/// `engine_white` is White, `engine_black` is Black.
pub fn play_engine_match(
    mut engine_white: Box<dyn Engine>,
    mut engine_black: Box<dyn Engine>,
    seed: u64,
    config: MatchConfig,
) -> Result<MatchResult, String> {
    play_engine_match_from_state_internal(
        GameState::new_game(),
        &mut engine_white,
        &mut engine_black,
        seed,
        config,
        true,
    )
}

/// Play a single seeded engine-vs-engine match from a caller-provided state.
///
/// This entrypoint bypasses random opening plies and is intended for curated
/// acceptance suites (for example endgame conversion testing).
pub fn play_engine_match_from_state(
    mut engine_white: Box<dyn Engine>,
    mut engine_black: Box<dyn Engine>,
    start_state: GameState,
    seed: u64,
    config: MatchConfig,
) -> Result<MatchResult, String> {
    play_engine_match_from_state_internal(
        start_state,
        &mut engine_white,
        &mut engine_black,
        seed,
        config,
        false,
    )
}

fn play_engine_match_from_state_internal(
    mut state: GameState,
    engine_white: &mut Box<dyn Engine>,
    engine_black: &mut Box<dyn Engine>,
    seed: u64,
    config: MatchConfig,
    apply_random_opening: bool,
) -> Result<MatchResult, String> {
    engine_white.new_game();
    engine_black.new_game();

    let opening_moves_lan = if apply_random_opening {
        let (state_after_opening, opening_moves_lan) = apply_seeded_random_opening(
            &state,
            seed,
            config.opening_min_plies,
            config.opening_max_plies,
        )?;
        state = state_after_opening;
        opening_moves_lan
    } else {
        Vec::new()
    };

    let mut played_moves_lan = Vec::<String>::new();
    let mut white_move_count = 0u32;
    let mut black_move_count = 0u32;
    let mut white_total_time_ns = 0u128;
    let mut black_total_time_ns = 0u128;

    for _ in 0..config.max_plies {
        if state.halfmove_clock >= 100 {
            return Ok(MatchResult {
                outcome: MatchOutcome::DrawFiftyMoveRule,
                final_state: state,
                opening_moves_lan,
                played_moves_lan,
                white_move_count,
                black_move_count,
                white_total_time_ns,
                black_total_time_ns,
            });
        }

        if repetition_count(&state) >= 3 {
            return Ok(MatchResult {
                outcome: MatchOutcome::DrawRepetition,
                final_state: state,
                opening_moves_lan,
                played_moves_lan,
                white_move_count,
                black_move_count,
                white_total_time_ns,
                black_total_time_ns,
            });
        }

        let mut probe = state.clone();
        let legal_moves = generate_legal_move_descriptions_in_place(&mut probe)
            .map_err(|e| format!("failed to generate legal moves: {e}"))?;
        if legal_moves.is_empty() {
            let outcome = if is_king_in_check(&state, state.side_to_move) {
                match state.side_to_move {
                    Color::Light => MatchOutcome::BlackWinCheckmate,
                    Color::Dark => MatchOutcome::WhiteWinCheckmate,
                }
            } else {
                MatchOutcome::DrawStalemate
            };
            return Ok(MatchResult {
                outcome,
                final_state: state,
                opening_moves_lan,
                played_moves_lan,
                white_move_count,
                black_move_count,
                white_total_time_ns,
                black_total_time_ns,
            });
        }

        let mover = state.side_to_move;
        let started = Instant::now();
        let out = if mover == Color::Light {
            engine_white.choose_move(&state, &config.go_params)?
        } else {
            engine_black.choose_move(&state, &config.go_params)?
        };
        let elapsed_ns = started.elapsed().as_nanos();

        match mover {
            Color::Light => {
                white_move_count = white_move_count.saturating_add(1);
                white_total_time_ns = white_total_time_ns.saturating_add(elapsed_ns);
            }
            Color::Dark => {
                black_move_count = black_move_count.saturating_add(1);
                black_total_time_ns = black_total_time_ns.saturating_add(elapsed_ns);
            }
        }

        let chosen = out.best_move.unwrap_or(legal_moves[0]);
        if !legal_moves.contains(&chosen) {
            return Err("engine returned illegal move".to_owned());
        }

        let lan = move_description_to_long_algebraic(chosen, &state)?;
        played_moves_lan.push(lan);
        state = apply_move(&state, chosen)?;
    }

    Ok(MatchResult {
        outcome: MatchOutcome::DrawMaxPlies,
        final_state: state,
        opening_moves_lan,
        played_moves_lan,
        white_move_count,
        black_move_count,
        white_total_time_ns,
        black_total_time_ns,
    })
}

/// Play a series of matches and aggregate win/loss/draw statistics.
///
/// Player colors are randomized each game (deterministic from `base_seed`).
pub fn play_engine_match_series<F1, F2>(
    player1_factory: F1,
    player2_factory: F2,
    config: MatchSeriesConfig,
) -> Result<MatchSeriesStats, String>
where
    F1: Fn() -> Box<dyn Engine>,
    F2: Fn() -> Box<dyn Engine>,
{
    let mut stats = MatchSeriesStats {
        games: config.games,
        ..MatchSeriesStats::default()
    };
    let mut color_rng = StdRng::seed_from_u64(config.base_seed ^ 0xA5A5_5A5A_0123_4567);

    for i in 0..config.games {
        let player1_is_white = color_rng.random_bool(0.5);
        let seed = config.base_seed.wrapping_add(u64::from(i));
        if config.verbose {
            let (white, black) = if player1_is_white {
                ("Player1", "Player2")
            } else {
                ("Player2", "Player1")
            };
            println!(
                "[series] game {}/{} seed={} white={} black={}",
                i + 1,
                config.games,
                seed,
                white,
                black
            );
        }

        let result = if player1_is_white {
            play_engine_match(
                player1_factory(),
                player2_factory(),
                seed,
                config.per_game.clone(),
            )?
        } else {
            play_engine_match(
                player2_factory(),
                player1_factory(),
                seed,
                config.per_game.clone(),
            )?
        };

        if player1_is_white {
            stats.player1_moves = stats.player1_moves.saturating_add(result.white_move_count);
            stats.player2_moves = stats.player2_moves.saturating_add(result.black_move_count);
            stats.player1_total_time_ns = stats
                .player1_total_time_ns
                .saturating_add(result.white_total_time_ns);
            stats.player2_total_time_ns = stats
                .player2_total_time_ns
                .saturating_add(result.black_total_time_ns);
        } else {
            stats.player1_moves = stats.player1_moves.saturating_add(result.black_move_count);
            stats.player2_moves = stats.player2_moves.saturating_add(result.white_move_count);
            stats.player1_total_time_ns = stats
                .player1_total_time_ns
                .saturating_add(result.black_total_time_ns);
            stats.player2_total_time_ns = stats
                .player2_total_time_ns
                .saturating_add(result.white_total_time_ns);
        }

        let mapped = match result.outcome {
            MatchOutcome::WhiteWinCheckmate => {
                if player1_is_white {
                    stats.player1_wins += 1;
                    SeriesOutcome::PlayerWinCheckmate {
                        player: PlayerId::Player1,
                        color: Color::Light,
                    }
                } else {
                    stats.player2_wins += 1;
                    SeriesOutcome::PlayerWinCheckmate {
                        player: PlayerId::Player2,
                        color: Color::Light,
                    }
                }
            }
            MatchOutcome::BlackWinCheckmate => {
                if player1_is_white {
                    stats.player2_wins += 1;
                    SeriesOutcome::PlayerWinCheckmate {
                        player: PlayerId::Player2,
                        color: Color::Dark,
                    }
                } else {
                    stats.player1_wins += 1;
                    SeriesOutcome::PlayerWinCheckmate {
                        player: PlayerId::Player1,
                        color: Color::Dark,
                    }
                }
            }
            MatchOutcome::DrawStalemate => {
                stats.draws += 1;
                SeriesOutcome::DrawStalemate
            }
            MatchOutcome::DrawRepetition => {
                stats.draws += 1;
                SeriesOutcome::DrawRepetition
            }
            MatchOutcome::DrawFiftyMoveRule => {
                stats.draws += 1;
                SeriesOutcome::DrawFiftyMoveRule
            }
            MatchOutcome::DrawMaxPlies => {
                stats.draws += 1;
                SeriesOutcome::DrawMaxPlies
            }
        };
        stats.outcomes.push(mapped);

        if config.verbose {
            let latest = stats
                .outcomes
                .last()
                .map(|o| format!("{:?}", o))
                .unwrap_or_else(|| "unknown".to_owned());
            println!(
                "[series] game {}/{} result={} p1_wins={} p2_wins={} draws={}\n",
                i + 1,
                config.games,
                latest,
                stats.player1_wins,
                stats.player2_wins,
                stats.draws
            );
        }
    }

    stats.player1_avg_move_time_ms =
        avg_ns_per_move_ms(stats.player1_total_time_ns, stats.player1_moves);
    stats.player2_avg_move_time_ms =
        avg_ns_per_move_ms(stats.player2_total_time_ns, stats.player2_moves);

    let total_ns = stats
        .player1_total_time_ns
        .saturating_add(stats.player2_total_time_ns);
    let total_moves = stats.player1_moves.saturating_add(stats.player2_moves);
    stats.overall_avg_move_time_ms = avg_ns_per_move_ms(total_ns, total_moves);

    Ok(stats)
}

#[inline]
fn avg_ns_per_move_ms(total_ns: u128, moves: u32) -> f64 {
    if moves == 0 {
        0.0
    } else {
        (total_ns as f64) / (moves as f64) / 1_000_000.0
    }
}

fn apply_seeded_random_opening(
    initial: &GameState,
    seed: u64,
    min_plies: u8,
    max_plies: u8,
) -> Result<(GameState, Vec<String>), String> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut state = initial.clone();
    let mut opening_moves_lan = Vec::<String>::new();
    let book = OpeningBook::load_default();

    let low = min_plies.min(max_plies);
    let high = max_plies.max(min_plies);
    let target_plies = if low == high {
        low
    } else {
        rng.random_range(low..=high)
    };

    for _ in 0..target_plies {
        let mut probe = state.clone();
        let legal_moves = generate_legal_move_descriptions_in_place(&mut probe)
            .map_err(|e| format!("failed to generate legal opening moves: {e}"))?;
        if legal_moves.is_empty() {
            break;
        }

        let mut chosen = book
            .choose_weighted_move(&state, &mut rng)
            .filter(|m| legal_moves.contains(m));
        if chosen.is_none() {
            let idx = rng.random_range(0..legal_moves.len());
            chosen = Some(legal_moves[idx]);
        }
        let chosen = chosen.expect("chosen move should exist");

        let lan = move_description_to_long_algebraic(chosen, &state)?;
        opening_moves_lan.push(lan);
        state = apply_move(&state, chosen)?;
    }

    Ok((state, opening_moves_lan))
}

#[inline]
fn repetition_count(state: &GameState) -> usize {
    let current = state.zobrist_key;
    state
        .repetition_history
        .iter()
        .filter(|h| **h == current)
        .count()
}

#[cfg(test)]
mod tests {
    use super::{
        play_engine_match, play_engine_match_series, MatchConfig, MatchOutcome, MatchSeriesConfig,
        SeriesOutcome,
    };
    use crate::engines::engine_greedy::GreedyEngine;
    use crate::engines::engine_random::RandomEngine;
    use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
    use crate::game_state::game_state::GameState;
    use crate::move_generation::legal_move_generator::FastLegalMoveGenerator;
    use crate::search::board_scoring::{AlphaZeroMetric, BoardScorer, MaterialScorer};
    use crate::search::iterative_deepening::{iterative_deepening_search, SearchConfig};

    struct ConfigurableIterativeTestEngine<S: BoardScorer + Clone + Send + Sync + 'static> {
        scorer: S,
        depth: u8,
        generator: FastLegalMoveGenerator,
    }

    impl<S: BoardScorer + Clone + Send + Sync + 'static> ConfigurableIterativeTestEngine<S> {
        fn new(scorer: S, depth: u8) -> Self {
            Self {
                scorer,
                depth,
                generator: FastLegalMoveGenerator,
            }
        }
    }

    impl<S: BoardScorer + Clone + Send + Sync + 'static> Engine for ConfigurableIterativeTestEngine<S> {
        fn choose_move(
            &mut self,
            game_state: &GameState,
            params: &GoParams,
        ) -> Result<EngineOutput, String> {
            let depth = params.depth.unwrap_or(self.depth).max(1);
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
                ponder_move: None,
                info_lines: vec![format!("info string test_engine depth {}", depth)],
            })
        }
    }

    #[test]
    fn engine_match_harness_runs_random_vs_greedy() {
        let white = Box::new(RandomEngine::new());
        let black = Box::new(GreedyEngine::new());
        let result = play_engine_match(
            white,
            black,
            42,
            MatchConfig {
                max_plies: 40,
                opening_min_plies: 2,
                opening_max_plies: 6,
                ..MatchConfig::default()
            },
        )
        .expect("match should run");

        assert!(!result.opening_moves_lan.is_empty());
        assert!(result.white_move_count + result.black_move_count > 0);
        assert!(matches!(
            result.outcome,
            MatchOutcome::WhiteWinCheckmate
                | MatchOutcome::BlackWinCheckmate
                | MatchOutcome::DrawStalemate
                | MatchOutcome::DrawRepetition
                | MatchOutcome::DrawFiftyMoveRule
                | MatchOutcome::DrawMaxPlies
        ));
    }

    #[test]
    fn engine_match_series_can_use_custom_scorer_and_depth_per_player() {
        let stats = play_engine_match_series(
            || Box::new(ConfigurableIterativeTestEngine::new(AlphaZeroMetric, 1)),
            || Box::new(ConfigurableIterativeTestEngine::new(MaterialScorer, 2)),
            MatchSeriesConfig {
                games: 3,
                base_seed: 777,
                per_game: MatchConfig {
                    max_plies: 16,
                    opening_min_plies: 2,
                    opening_max_plies: 4,
                    ..MatchConfig::default()
                },
                verbose: false,
            },
        )
        .expect("series should run");

        assert_eq!(stats.games, 3);
        assert_eq!(stats.outcomes.len(), 3);
        assert!(stats.player1_avg_move_time_ms >= 0.0);
        assert!(stats.player2_avg_move_time_ms >= 0.0);
        assert!(stats.overall_avg_move_time_ms >= 0.0);
        assert!(stats.outcomes.iter().all(|o| {
            matches!(
                o,
                SeriesOutcome::PlayerWinCheckmate { .. }
                    | SeriesOutcome::DrawStalemate
                    | SeriesOutcome::DrawRepetition
                    | SeriesOutcome::DrawFiftyMoveRule
                    | SeriesOutcome::DrawMaxPlies
            )
        }));
    }
}
