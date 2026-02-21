//! Humanized CPL engine scaffold for major version 5.
//!
//! This module starts with a compatibility wrapper around v16 iterative search
//! and will progressively add CPL-based top-candidate selection behavior.

use crate::engines::engine_iterative_v16::IterativeEngine;
use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use std::sync::{atomic::AtomicBool, Arc};

pub struct HumanizedEngineV5 {
    #[allow(dead_code)]
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
        self.inner.choose_move(game_state, params)
    }
}

#[cfg(test)]
mod tests {
    use super::{allowed_cpl_loss, cpl_bounds, default_depth_for_level, strength_percent};

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
}
