//! Iterative engine V17 scaffold (major version 6).
//!
//! This module is the v6 endpoint for endgame-strength upgrades.
//! In v6.0 it intentionally delegates to v16 behavior while preserving
//! compatibility and adding explicit version markers for A/B rollout testing.

use crate::engines::engine_iterative_v16::{IterativeEngine as IterativeEngineV16, IterativeScorerKind};
use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use std::sync::{atomic::AtomicBool, Arc};

pub struct IterativeEngineV17 {
    inner: IterativeEngineV16,
}

impl IterativeEngineV17 {
    pub fn new(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::AlphaZero)
    }

    pub fn new_standard(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::Standard)
    }

    pub fn new_alpha_zero(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::AlphaZero)
    }

    pub fn new_with_scorer(default_depth: u8, scorer_kind: IterativeScorerKind) -> Self {
        Self {
            inner: IterativeEngineV16::new_with_scorer(default_depth, scorer_kind),
        }
    }
}

impl Engine for IterativeEngineV17 {
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
        let mut out = self.inner.choose_move(game_state, params)?;
        out.info_lines
            .push("info string iterative_engine_v17 scaffold active".to_owned());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::IterativeEngineV17;
    use crate::engines::engine_trait::{Engine, GoParams};
    use crate::game_state::game_state::GameState;

    #[test]
    fn v17_scaffold_emits_marker_and_bestmove() {
        let game = GameState::new_game();
        let mut engine = IterativeEngineV17::new_alpha_zero(2);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        let out = engine
            .choose_move(
                &game,
                &GoParams {
                    depth: Some(1),
                    ..GoParams::default()
                },
            )
            .expect("engine should choose a move");
        assert!(out.best_move.is_some());
        assert!(out
            .info_lines
            .iter()
            .any(|l| l.contains("iterative_engine_v17 scaffold active")));
    }
}
