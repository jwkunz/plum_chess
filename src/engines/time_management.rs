//! Reusable time-management strategies for engine move budgeting.
//!
//! UCI should pass raw clock data (`wtime/btime/winc/binc/movetime`) and the
//! engine should decide final per-move allocation based on strategy.

use crate::engines::engine_trait::GoParams;
use crate::game_state::chess_types::Color;
use crate::game_state::game_state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeManagementStrategy {
    /// Legacy fixed rule: spend 1/20th of remaining clock.
    Fraction20,
    /// Improved adaptive rule using clock, increment, and game phase.
    AdaptiveV13,
}

pub fn resolve_go_params(
    game_state: &GameState,
    params: &GoParams,
    strategy: TimeManagementStrategy,
) -> GoParams {
    if params.movetime_ms.is_some() {
        return params.clone();
    }

    let mut resolved = params.clone();
    let (remaining_opt, inc_opt) = match game_state.side_to_move {
        Color::Light => (params.wtime_ms, params.winc_ms),
        Color::Dark => (params.btime_ms, params.binc_ms),
    };

    if let Some(remaining) = remaining_opt {
        resolved.movetime_ms = Some(match strategy {
            TimeManagementStrategy::Fraction20 => (remaining / 20).max(1),
            TimeManagementStrategy::AdaptiveV13 => {
                adaptive_budget_ms(game_state, remaining, inc_opt, params.movestogo)
            }
        });
    }

    resolved
}

fn adaptive_budget_ms(
    game_state: &GameState,
    remaining_ms: u64,
    inc_ms: Option<u64>,
    movestogo: Option<u16>,
) -> u64 {
    let ply = u64::from(game_state.ply);
    let expected_moves_left = if let Some(mtg) = movestogo {
        u64::from(mtg.max(1))
    } else if ply < 20 {
        40
    } else if ply < 60 {
        28
    } else {
        18
    };

    let reserve = (remaining_ms / 25).clamp(100, remaining_ms.saturating_sub(1));
    let usable = remaining_ms.saturating_sub(reserve);
    let base = usable / expected_moves_left.max(1);
    let inc_bonus = inc_ms.unwrap_or(0).saturating_mul(3) / 4;
    let panic = if remaining_ms < 2_000 {
        remaining_ms / 12
    } else {
        0
    };
    let target = base.saturating_add(inc_bonus).saturating_add(panic);

    let min_budget = if remaining_ms < 1_000 { 5 } else { 15 };
    let max_budget = (remaining_ms / 4).max(1);
    target.clamp(min_budget, max_budget).max(1)
}
