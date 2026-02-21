//! Iterative engine V17 scaffold (major version 6).
//!
//! This module is the v6 endpoint for endgame-strength upgrades.
//! In v6.0 it intentionally delegates to v16 behavior while preserving
//! compatibility and adding explicit version markers for A/B rollout testing.

use crate::engines::engine_iterative_v16::{IterativeEngine as IterativeEngineV16, IterativeScorerKind};
use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::chess_types::{Color, PieceKind};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;
use crate::moves::move_descriptions::piece_kind_from_code;
use std::collections::{HashMap, HashSet};
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
        let in_kpk = kpk_pawn_color(game_state).is_some();
        if in_endgame_mode {
            out.info_lines
                .push("info string iterative_engine_v17 endgame_mode conservative".to_owned());
        }
        if in_kpk {
            if let Some(best) = select_kpk_best_move(game_state) {
                out.best_move = Some(best);
                out.info_lines
                    .push("info string iterative_engine_v17 kpk_exact_applied".to_owned());
            }
        }
        if let Some(best) = select_kbnk_best_move(game_state) {
            out.best_move = Some(best);
            out.info_lines
                .push("info string iterative_engine_v17 kbnk_logic_applied".to_owned());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KpkOutcome {
    Win,
    Draw,
    Loss,
}

#[inline]
fn invert_kpk(outcome: KpkOutcome) -> KpkOutcome {
    match outcome {
        KpkOutcome::Win => KpkOutcome::Loss,
        KpkOutcome::Loss => KpkOutcome::Win,
        KpkOutcome::Draw => KpkOutcome::Draw,
    }
}

fn kpk_pawn_color(game_state: &GameState) -> Option<Color> {
    let mut pawn_color = None;
    let mut non_king_non_pawn = 0u32;
    let mut pawn_count = 0u32;
    for color in [Color::Light, Color::Dark] {
        let idx = color.index();
        for piece_code in 0..6usize {
            let count = game_state.pieces[idx][piece_code].count_ones();
            if piece_code == PieceKind::Pawn.index() {
                if count > 0 {
                    pawn_color = Some(color);
                }
                pawn_count += count;
            } else if piece_code != PieceKind::King.index() {
                non_king_non_pawn += count;
            }
        }
    }
    if pawn_count == 1 && non_king_non_pawn == 0 {
        pawn_color
    } else {
        None
    }
}

fn select_kpk_best_move(game_state: &GameState) -> Option<u64> {
    let pawn_color = kpk_pawn_color(game_state)?;
    let mut probe = game_state.clone();
    let legal = generate_legal_move_descriptions_in_place(&mut probe).ok()?;
    let mut memo = HashMap::<u64, KpkOutcome>::new();
    let mut best = None;
    let mut best_outcome = KpkOutcome::Loss;

    for mv in legal {
        let Ok(next) = apply_move(game_state, mv) else {
            continue;
        };
        let outcome = if kpk_pawn_color(&next).is_none() {
            // Leaving KPK typically means promotion; treat as decisive
            // for the pawn side if that side still has non-pawn material.
            if has_promoted_material(&next, pawn_color) {
                if game_state.side_to_move == pawn_color {
                    KpkOutcome::Win
                } else {
                    KpkOutcome::Loss
                }
            } else {
                KpkOutcome::Draw
            }
        } else {
            let mut visiting = HashSet::<u64>::new();
            let child = solve_kpk_outcome(&next, &mut memo, &mut visiting, 0);
            invert_kpk(child)
        };

        if outcome == KpkOutcome::Win {
            return Some(mv);
        }
        if outcome == KpkOutcome::Draw && best_outcome == KpkOutcome::Loss {
            best = Some(mv);
            best_outcome = KpkOutcome::Draw;
        } else if best.is_none() {
            best = Some(mv);
        }
    }
    best
}

fn has_promoted_material(game_state: &GameState, color: Color) -> bool {
    let idx = color.index();
    game_state.pieces[idx][PieceKind::Queen.index()] != 0
        || game_state.pieces[idx][PieceKind::Rook.index()] != 0
        || game_state.pieces[idx][PieceKind::Bishop.index()] != 0
        || game_state.pieces[idx][PieceKind::Knight.index()] != 0
}

fn solve_kpk_outcome(
    game_state: &GameState,
    memo: &mut HashMap<u64, KpkOutcome>,
    visiting: &mut HashSet<u64>,
    depth: u16,
) -> KpkOutcome {
    if depth >= 96 {
        return KpkOutcome::Draw;
    }
    if let Some(&cached) = memo.get(&game_state.zobrist_key) {
        return cached;
    }
    if !visiting.insert(game_state.zobrist_key) {
        return KpkOutcome::Draw;
    }

    let mut probe = game_state.clone();
    let legal = match generate_legal_move_descriptions_in_place(&mut probe) {
        Ok(v) => v,
        Err(_) => {
            visiting.remove(&game_state.zobrist_key);
            return KpkOutcome::Draw;
        }
    };
    if legal.is_empty() {
        let outcome = if is_king_in_check(game_state, game_state.side_to_move) {
            KpkOutcome::Loss
        } else {
            KpkOutcome::Draw
        };
        memo.insert(game_state.zobrist_key, outcome);
        visiting.remove(&game_state.zobrist_key);
        return outcome;
    }

    let pawn_color = kpk_pawn_color(game_state);
    let mut saw_draw = false;
    for mv in legal {
        let Ok(next) = apply_move(game_state, mv) else {
            continue;
        };
        let child = if kpk_pawn_color(&next).is_none() {
            if let Some(pc) = pawn_color {
                if has_promoted_material(&next, pc) {
                    // `child` is from next-side-to-move perspective.
                    if game_state.side_to_move == pc {
                        KpkOutcome::Loss
                    } else {
                        KpkOutcome::Win
                    }
                } else {
                    KpkOutcome::Draw
                }
            } else {
                KpkOutcome::Draw
            }
        } else {
            solve_kpk_outcome(&next, memo, visiting, depth + 1)
        };
        let our = invert_kpk(child);
        if our == KpkOutcome::Win {
            memo.insert(game_state.zobrist_key, KpkOutcome::Win);
            visiting.remove(&game_state.zobrist_key);
            return KpkOutcome::Win;
        }
        if our == KpkOutcome::Draw {
            saw_draw = true;
        }
    }

    let result = if saw_draw {
        KpkOutcome::Draw
    } else {
        KpkOutcome::Loss
    };
    memo.insert(game_state.zobrist_key, result);
    visiting.remove(&game_state.zobrist_key);
    result
}

fn kbnk_attacker_color(game_state: &GameState) -> Option<Color> {
    let mut side_with_kbn = None;
    for color in [Color::Light, Color::Dark] {
        let idx = color.index();
        let bishops = game_state.pieces[idx][PieceKind::Bishop.index()].count_ones();
        let knights = game_state.pieces[idx][PieceKind::Knight.index()].count_ones();
        let pawns = game_state.pieces[idx][PieceKind::Pawn.index()].count_ones();
        let rooks = game_state.pieces[idx][PieceKind::Rook.index()].count_ones();
        let queens = game_state.pieces[idx][PieceKind::Queen.index()].count_ones();
        if bishops == 1 && knights == 1 && pawns == 0 && rooks == 0 && queens == 0 {
            side_with_kbn = Some(color);
        }
    }
    let attacker = side_with_kbn?;
    let defender = attacker.opposite();
    let d_idx = defender.index();
    let defender_non_king = game_state.pieces[d_idx][PieceKind::Pawn.index()].count_ones()
        + game_state.pieces[d_idx][PieceKind::Knight.index()].count_ones()
        + game_state.pieces[d_idx][PieceKind::Bishop.index()].count_ones()
        + game_state.pieces[d_idx][PieceKind::Rook.index()].count_ones()
        + game_state.pieces[d_idx][PieceKind::Queen.index()].count_ones();
    if defender_non_king == 0 {
        Some(attacker)
    } else {
        None
    }
}

fn select_kbnk_best_move(game_state: &GameState) -> Option<u64> {
    let attacker = kbnk_attacker_color(game_state)?;
    if game_state.side_to_move != attacker {
        return None;
    }
    let mut probe = game_state.clone();
    let legal = generate_legal_move_descriptions_in_place(&mut probe).ok()?;
    let mut best = None;
    let mut best_score = i32::MIN;
    for mv in legal {
        let Ok(next) = apply_move(game_state, mv) else {
            continue;
        };
        if is_mate_for_side_to_move(&next) {
            return Some(mv);
        }
        let score = kbnk_progress_score(&next, attacker);
        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
    }
    best
}

fn kbnk_progress_score(game_state: &GameState, attacker: Color) -> i32 {
    let defender = attacker.opposite();
    let mut probe = game_state.clone();
    let defender_moves = generate_legal_move_descriptions_in_place(&mut probe)
        .map(|v| v.len() as i32)
        .unwrap_or(32);
    let defender_king_sq = king_square(game_state, defender).unwrap_or(0);
    let attacker_king_sq = king_square(game_state, attacker).unwrap_or(0);
    let bishop_dark = bishop_on_dark_square(game_state, attacker).unwrap_or(true);
    let target_corners = if bishop_dark {
        [0u8, 63u8] // a1, h8
    } else {
        [7u8, 56u8] // h1, a8
    };
    let corner_dist = target_corners
        .iter()
        .map(|c| manhattan(defender_king_sq, *c))
        .min()
        .unwrap_or(14) as i32;
    let king_dist = manhattan(attacker_king_sq, defender_king_sq) as i32;
    let in_target_corner = target_corners.contains(&defender_king_sq);
    let mut score = 0i32;
    score -= corner_dist * 24;
    score -= defender_moves * 18;
    score -= king_dist * 7;
    if in_target_corner {
        score += 80;
    }
    score
}

fn bishop_on_dark_square(game_state: &GameState, color: Color) -> Option<bool> {
    let bb = game_state.pieces[color.index()][PieceKind::Bishop.index()];
    if bb == 0 {
        return None;
    }
    let sq = bb.trailing_zeros() as u8;
    Some(is_dark_square(sq))
}

fn king_square(game_state: &GameState, color: Color) -> Option<u8> {
    let bb = game_state.pieces[color.index()][PieceKind::King.index()];
    if bb == 0 {
        None
    } else {
        Some(bb.trailing_zeros() as u8)
    }
}

#[inline]
fn is_dark_square(square: u8) -> bool {
    let file = square % 8;
    let rank = square / 8;
    (file + rank) % 2 == 0
}

#[inline]
fn manhattan(a: u8, b: u8) -> u8 {
    let af = (a % 8) as i8;
    let ar = (a / 8) as i8;
    let bf = (b % 8) as i8;
    let br = (b / 8) as i8;
    ((af - bf).abs() + (ar - br).abs()) as u8
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

    #[test]
    fn kpk_exact_detects_simple_win() {
        let game = crate::utils::fen_parser::parse_fen("k7/7P/8/8/8/8/8/K7 w - - 0 1")
            .expect("fen should parse");
        assert!(super::kpk_pawn_color(&game).is_some());
        let mut memo = std::collections::HashMap::new();
        let mut visiting = std::collections::HashSet::new();
        let outcome = super::solve_kpk_outcome(&game, &mut memo, &mut visiting, 0);
        assert_eq!(outcome, super::KpkOutcome::Win);
    }

    #[test]
    fn kpk_exact_detects_simple_draw() {
        let game = crate::utils::fen_parser::parse_fen("8/8/8/8/4k3/8/4P3/4K3 w - - 0 1")
            .expect("fen should parse");
        assert!(super::kpk_pawn_color(&game).is_some());
        let mut memo = std::collections::HashMap::new();
        let mut visiting = std::collections::HashSet::new();
        let outcome = super::solve_kpk_outcome(&game, &mut memo, &mut visiting, 0);
        assert_eq!(outcome, super::KpkOutcome::Draw);
    }

    #[test]
    fn kbnk_detection_and_selection_work() {
        let game = crate::utils::fen_parser::parse_fen("8/8/8/8/8/8/4KB2/6Nk w - - 0 1")
            .expect("fen should parse");
        assert_eq!(
            super::kbnk_attacker_color(&game),
            Some(crate::game_state::chess_types::Color::Light)
        );
        let best = super::select_kbnk_best_move(&game);
        assert!(best.is_some());
    }
}
