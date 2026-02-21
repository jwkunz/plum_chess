//! Iterative engine V17 scaffold (major version 6).
//!
//! This module is the v6 endpoint for endgame-strength upgrades.
//! In v6.0 it intentionally delegates to v16 behavior while preserving
//! compatibility and adding explicit version markers for A/B rollout testing.

use crate::engines::engine_iterative_v16::{IterativeEngine as IterativeEngineV16, IterativeScorerKind};
use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::moves::move_descriptions::piece_kind_from_code;
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
        let in_endgame_mode = is_conservative_endgame(game_state);
        if in_endgame_mode {
            out.info_lines
                .push("info string iterative_engine_v17 endgame_mode conservative".to_owned());
        }
        let winning_cp = extract_last_cp_score(&out.info_lines).unwrap_or(0);
        if in_endgame_mode && winning_cp >= 200 {
            if let Some(chosen) = out.best_move {
                if would_be_threefold_after_move(game_state, chosen) {
                    let mut probe = game_state.clone();
                    let legal = generate_legal_move_descriptions_in_place(&mut probe)
                        .map_err(|e| e.to_string())?;
                    if let Some(replacement) =
                        select_non_repetition_best_material_move(game_state, &legal, chosen)
                    {
                        out.best_move = Some(replacement);
                        out.info_lines.push(
                            "info string iterative_engine_v17 strong_draw_avoidance_applied"
                                .to_owned(),
                        );
                    }
                }
            }
        }
        out.info_lines
            .push("info string iterative_engine_v17 scaffold active".to_owned());
        Ok(out)
    }
}

fn extract_last_cp_score(info_lines: &[String]) -> Option<i32> {
    for line in info_lines.iter().rev() {
        if let Some(idx) = line.find(" score cp ") {
            let cp_part = &line[(idx + " score cp ".len())..];
            if let Some(cp_tok) = cp_part.split_whitespace().next() {
                if let Ok(cp) = cp_tok.parse::<i32>() {
                    return Some(cp);
                }
            }
        }
    }
    None
}

fn would_be_threefold_after_move(game_state: &GameState, mv: u64) -> bool {
    let Ok(next) = apply_move(game_state, mv) else {
        return false;
    };
    let current = next.zobrist_key;
    next.repetition_history
        .iter()
        .filter(|&&k| k == current)
        .count()
        >= 3
}

fn select_non_repetition_best_material_move(
    game_state: &GameState,
    legal_moves: &[u64],
    chosen: u64,
) -> Option<u64> {
    let mut best_alt = None;
    let mut best_score = i32::MIN;
    for &mv in legal_moves {
        if mv == chosen || would_be_threefold_after_move(game_state, mv) {
            continue;
        }
        let Ok(next) = apply_move(game_state, mv) else {
            continue;
        };
        if is_mate_for_side_to_move(&next) {
            return Some(mv);
        }
        let score = material_score_for_color(&next, game_state.side_to_move);
        if score > best_score {
            best_score = score;
            best_alt = Some(mv);
        }
    }
    best_alt
}

fn is_mate_for_side_to_move(next: &GameState) -> bool {
    let mut probe = next.clone();
    let Ok(replies) = generate_legal_move_descriptions_in_place(&mut probe) else {
        return false;
    };
    replies.is_empty() && is_king_in_check(next, next.side_to_move)
}

fn material_score_for_color(game_state: &GameState, color: crate::game_state::chess_types::Color) -> i32 {
    let us = color.index();
    let them = color.opposite().index();
    let mut score = 0i32;
    for piece_code in 0u8..6u8 {
        let Some(kind) = piece_kind_from_code(u64::from(piece_code)) else {
            continue;
        };
        let value = match kind {
            crate::game_state::chess_types::PieceKind::Pawn => 100,
            crate::game_state::chess_types::PieceKind::Knight => 350,
            crate::game_state::chess_types::PieceKind::Bishop => 325,
            crate::game_state::chess_types::PieceKind::Rook => 500,
            crate::game_state::chess_types::PieceKind::Queen => 975,
            crate::game_state::chess_types::PieceKind::King => 0,
        };
        score += (game_state.pieces[us][piece_code as usize].count_ones() as i32) * value;
        score -= (game_state.pieces[them][piece_code as usize].count_ones() as i32) * value;
    }
    score
}

fn is_conservative_endgame(game_state: &GameState) -> bool {
    let mut non_king_count = 0u32;
    let mut queen_count = 0u32;
    for color in [crate::game_state::chess_types::Color::Light, crate::game_state::chess_types::Color::Dark] {
        let idx = color.index();
        for piece_code in 0..6usize {
            let count = game_state.pieces[idx][piece_code].count_ones();
            if piece_code != crate::game_state::chess_types::PieceKind::King.index() {
                non_king_count += count;
            }
            if piece_code == crate::game_state::chess_types::PieceKind::Queen.index() {
                queen_count += count;
            }
        }
    }
    // Conservative gate: very low material, and at most one queen on board.
    non_king_count <= 6 && queen_count <= 1
}

#[cfg(test)]
mod tests {
    use super::IterativeEngineV17;
    use crate::engines::engine_trait::{Engine, GoParams};
    use crate::game_state::game_state::GameState;
    use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;

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

    #[test]
    fn threefold_detection_helper_detects_repetition() {
        let mut game = GameState::new_game();
        let mut probe = game.clone();
        let legal = generate_legal_move_descriptions_in_place(&mut probe).expect("legal");
        let mv = legal[0];
        let next = crate::move_generation::legal_move_apply::apply_move(&game, mv).expect("apply");
        game.repetition_history = vec![next.zobrist_key, next.zobrist_key];
        assert!(super::would_be_threefold_after_move(&game, mv));
    }

    #[test]
    fn conservative_endgame_trigger_detects_low_material() {
        let start = GameState::new_game();
        assert!(!super::is_conservative_endgame(&start));
        let low = crate::utils::fen_parser::parse_fen("8/8/8/8/8/8/5k2/6KR w - - 0 1")
            .expect("fen should parse");
        assert!(super::is_conservative_endgame(&low));
    }
}
