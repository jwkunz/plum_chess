//! Humanized CPL engine scaffold for major version 5.
//!
//! This module starts with a compatibility wrapper around v16 iterative search
//! and will progressively add CPL-based top-candidate selection behavior.

use crate::engines::engine_iterative_v16::IterativeEngine;
use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::utils::long_algebraic::long_algebraic_to_move_description;
use rand::Rng;
use std::sync::{atomic::AtomicBool, Arc};

pub struct HumanizedEngineV5 {
    level: u8,
    inner: IterativeEngine,
}

impl HumanizedEngineV5 {
    pub fn new(level: u8) -> Self {
        let default_depth = default_depth_for_level(level);
        Self {
            level,
            inner: IterativeEngine::new_standard(default_depth),
        }
    }
}

fn default_depth_for_level(level: u8) -> u8 {
    match level {
        3..=5 => 2,
        6..=8 => 3,
        9..=11 => 4,
        12..=14 => 5,
        15..=17 => 6,
        _ => 4,
    }
}

fn strength_percent(level: u8) -> f64 {
    // Linear map: level 3 -> 60%, level 17 -> 100%.
    if level <= 3 {
        return 0.60;
    }
    if level >= 17 {
        return 1.0;
    }
    let t = f64::from(level - 3) / f64::from(17 - 3);
    0.60 + (0.40 * t)
}

fn cpl_bounds(percent: f64) -> (i32, i32) {
    // Min bound shaped by sqrt(percent); max bound shaped by percent^2.
    // Lower strength -> larger bounds.
    let p = percent.clamp(0.0, 1.0);
    let min_cpl = ((1.0 - p.sqrt()) * 80.0).round() as i32;
    let max_cpl = ((1.0 - (p * p)) * 500.0).round() as i32;
    (min_cpl.max(0), max_cpl.max(min_cpl.max(0)))
}

fn allowed_cpl_loss(percent: f64) -> i32 {
    let p = percent.clamp(0.0, 1.0);
    let (_, max_cpl) = cpl_bounds(p);
    let raw = (f64::from(max_cpl) * (1.0 - p)).round() as i32;
    let (min_cpl, max_cpl) = cpl_bounds(p);
    raw.clamp(min_cpl, max_cpl)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScoredMove {
    mv: u64,
    cp: i32,
}

fn parse_multipv_candidates(info_lines: &[String], game_state: &GameState) -> Vec<ScoredMove> {
    let mut out = Vec::<ScoredMove>::new();
    for line in info_lines {
        if !line.contains(" multipv ") || !line.contains(" score cp ") || !line.contains(" pv ") {
            continue;
        }
        let Some(cp_idx) = line.find(" score cp ") else {
            continue;
        };
        let cp_part = &line[(cp_idx + " score cp ".len())..];
        let Some(cp_token) = cp_part.split_whitespace().next() else {
            continue;
        };
        let Ok(cp) = cp_token.parse::<i32>() else {
            continue;
        };
        let Some(pv_idx) = line.find(" pv ") else {
            continue;
        };
        let pv_part = &line[(pv_idx + " pv ".len())..];
        let Some(first_lan) = pv_part.split_whitespace().next() else {
            continue;
        };
        let Ok(mv) = long_algebraic_to_move_description(first_lan, game_state) else {
            continue;
        };
        if out.iter().any(|x| x.mv == mv) {
            continue;
        }
        out.push(ScoredMove { mv, cp });
    }
    out.sort_by(|a, b| b.cp.cmp(&a.cp));
    out
}

fn choose_humanized_move(
    candidates: &[ScoredMove],
    level: u8,
    rng: &mut impl Rng,
) -> Option<(u64, i32, i32)> {
    if candidates.len() < 3 {
        return None;
    }
    let best = candidates[0];
    let percent = strength_percent(level);
    let allowed = allowed_cpl_loss(percent);

    let mut allowed_moves = Vec::<(ScoredMove, i32)>::new();
    for c in candidates {
        let loss = (best.cp - c.cp).max(0);
        if loss <= allowed {
            allowed_moves.push((*c, loss));
        }
    }
    if allowed_moves.is_empty() {
        return Some((best.mv, 0, allowed));
    }

    let mut total_weight = 0u64;
    let mut weighted = Vec::<(ScoredMove, i32, u64)>::new();
    for (c, loss) in allowed_moves {
        let w = u64::from((allowed - loss + 1).max(1) as u32);
        total_weight += w;
        weighted.push((c, loss, w));
    }
    let mut pick = rng.random_range(0..total_weight);
    for (c, loss, w) in weighted {
        if pick < w {
            return Some((c.mv, loss, allowed));
        }
        pick -= w;
    }

    Some((best.mv, 0, allowed))
}

impl Engine for HumanizedEngineV5 {
    fn new_game(&mut self) {
        self.inner.new_game();
    }

    fn set_option(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.inner.set_option(name, value)
    }

    fn set_stop_signal(&mut self, stop_signal: Option<Arc<AtomicBool>>) {
        self.inner.set_stop_signal(stop_signal);
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        // Keep behavior after opening-book handling by letting the underlying
        // engine make that choice first.
        let _ = self.inner.set_option("MultiPV", "3");
        let mut out = self.inner.choose_move(game_state, params)?;
        if out
            .info_lines
            .iter()
            .any(|l| l.contains("opening book move"))
        {
            return Ok(out);
        }

        let candidates = parse_multipv_candidates(&out.info_lines, game_state);
        let mut rng = rand::rng();
        if let Some((mv, loss, allowed)) = choose_humanized_move(&candidates, self.level, &mut rng)
        {
            out.best_move = Some(mv);
            out.info_lines.push(format!(
                "info string humanized_v5 selected_cpl_loss {} allowed_cpl {} level {}",
                loss, allowed, self.level
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        allowed_cpl_loss, choose_humanized_move, cpl_bounds, default_depth_for_level,
        parse_multipv_candidates, strength_percent, HumanizedEngineV5, ScoredMove,
    };
    use crate::engines::engine_trait::{Engine, GoParams};
    use crate::game_state::game_state::GameState;
    use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
    use rand::SeedableRng;

    #[test]
    fn v5_default_depth_scales_across_level_bands() {
        assert_eq!(default_depth_for_level(3), 2);
        assert_eq!(default_depth_for_level(7), 3);
        assert_eq!(default_depth_for_level(10), 4);
        assert_eq!(default_depth_for_level(13), 5);
        assert_eq!(default_depth_for_level(16), 6);
    }

    #[test]
    fn strength_percent_maps_linearly_3_to_17() {
        assert!((strength_percent(3) - 0.60).abs() < 1e-9);
        assert!((strength_percent(10) - 0.80).abs() < 1e-9);
        assert!((strength_percent(17) - 1.00).abs() < 1e-9);
    }

    #[test]
    fn cpl_bounds_use_required_shaping_terms() {
        let (min_60, max_60) = cpl_bounds(0.60);
        let (min_100, max_100) = cpl_bounds(1.0);
        assert!(min_60 > min_100);
        assert!(max_60 > max_100);
        assert_eq!(min_100, 0);
        assert_eq!(max_100, 0);
    }

    #[test]
    fn allowed_cpl_loss_uses_confirmed_formula_with_clamps() {
        let low = allowed_cpl_loss(0.60);
        let high = allowed_cpl_loss(1.0);
        assert!(low > high);
        assert_eq!(high, 0);
    }

    #[test]
    fn parse_multipv_candidates_extracts_scored_moves() {
        let game = GameState::new_game();
        let lines = vec![
            "info depth 3 multipv 1 score cp 52 pv e2e4 e7e5".to_owned(),
            "info depth 3 multipv 2 score cp 48 pv d2d4 d7d5".to_owned(),
            "info depth 3 multipv 3 score cp 39 pv g1f3 g8f6".to_owned(),
        ];
        let parsed = parse_multipv_candidates(&lines, &game);
        assert_eq!(parsed.len(), 3);
        assert!(parsed[0].cp >= parsed[1].cp);
    }

    #[test]
    fn choose_humanized_move_returns_none_when_fewer_than_three() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let candidates = vec![ScoredMove { mv: 1, cp: 10 }, ScoredMove { mv: 2, cp: 8 }];
        assert!(choose_humanized_move(&candidates, 10, &mut rng).is_none());
    }

    #[test]
    fn choose_humanized_move_prefers_within_allowed_loss() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(9);
        let candidates = vec![
            ScoredMove { mv: 10, cp: 100 },
            ScoredMove { mv: 20, cp: 95 },
            ScoredMove { mv: 30, cp: 80 },
        ];
        let chosen = choose_humanized_move(&candidates, 17, &mut rng).expect("should choose");
        assert_eq!(chosen.0, 10);
        assert_eq!(chosen.2, 0);
    }

    #[test]
    fn engine_returns_legal_move_on_static_position() {
        let game = GameState::new_game();
        let mut engine = HumanizedEngineV5::new(10);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        let out = engine
            .choose_move(
                &game,
                &GoParams {
                    depth: Some(2),
                    ..GoParams::default()
                },
            )
            .expect("engine should choose move");
        let best = out.best_move.expect("best move");
        let mut probe = game.clone();
        let legal = generate_legal_move_descriptions_in_place(&mut probe).expect("legal moves");
        assert!(legal.contains(&best));
    }

    #[test]
    fn engine_returns_legal_moves_for_multiple_static_fens() {
        let fens = [
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ];
        let mut engine = HumanizedEngineV5::new(12);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        for fen in fens {
            let game = crate::utils::fen_parser::parse_fen(fen).expect("fen should parse");
            let out = engine
                .choose_move(
                    &game,
                    &GoParams {
                        depth: Some(2),
                        ..GoParams::default()
                    },
                )
                .expect("engine should choose");
            let best = out.best_move.expect("best move");
            let mut probe = game.clone();
            let legal =
                generate_legal_move_descriptions_in_place(&mut probe).expect("legal moves");
            assert!(legal.contains(&best));
        }
    }
}
