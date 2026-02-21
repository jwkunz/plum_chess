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
    use super::default_depth_for_level;

    #[test]
    fn v5_default_depth_scales_across_level_bands() {
        assert_eq!(default_depth_for_level(3), 2);
        assert_eq!(default_depth_for_level(7), 3);
        assert_eq!(default_depth_for_level(10), 4);
        assert_eq!(default_depth_for_level(13), 5);
        assert_eq!(default_depth_for_level(16), 6);
    }
}
