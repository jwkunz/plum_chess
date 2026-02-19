//! Iterative-deepening material-search engine (V10).
//!
//! Wraps the core negamax alpha-beta search with fixed-depth configuration and
//! material scoring for deterministic stronger difficulty levels.
//!
//! V10 marker:
//! - Successor to V9 with safer aggressive pruning.
//! - Adds null-move verification search in the backend.

use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::{
    generate_legal_move_descriptions_in_place, FastLegalMoveGenerator,
};
use crate::moves::move_descriptions::{
    move_from, move_promotion_piece_code, move_to, piece_kind_from_code,
};
use crate::search::board_scoring::{EndgameTaperedScorerV3, V3MaterialKind};
use crate::search::iterative_deepening_v10::{
    iterative_deepening_search_with_tt, principal_variation_from_tt, SearchConfig,
};
use crate::search::transposition_table::TranspositionTable;
use crate::tables::opening_book::OpeningBook;
use crate::utils::long_algebraic::move_description_to_long_algebraic;
use rand::rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterativeScorerKind {
    Standard,
    AlphaZero,
}

pub struct IterativeEngine {
    default_depth: u8,
    move_generator: FastLegalMoveGenerator,
    standard_scorer: EndgameTaperedScorerV3,
    alpha_zero_scorer: EndgameTaperedScorerV3,
    scorer_kind: IterativeScorerKind,
    opening_book: OpeningBook,
    use_own_book: bool,
    tt: TranspositionTable,
    hash_mb: usize,
}

impl IterativeEngine {
    pub fn new(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::Standard)
    }

    pub fn new_standard(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::Standard)
    }

    pub fn new_alpha_zero(default_depth: u8) -> Self {
        Self::new_with_scorer(default_depth, IterativeScorerKind::AlphaZero)
    }

    pub fn new_with_scorer(default_depth: u8, scorer_kind: IterativeScorerKind) -> Self {
        let hash_mb = 64usize;
        Self {
            default_depth,
            move_generator: FastLegalMoveGenerator,
            standard_scorer: EndgameTaperedScorerV3::standard(),
            alpha_zero_scorer: EndgameTaperedScorerV3::alpha_zero(),
            scorer_kind,
            opening_book: OpeningBook::load_default(),
            use_own_book: true,
            tt: TranspositionTable::new_with_mb(hash_mb),
            hash_mb,
        }
    }
}

impl Engine for IterativeEngine {
    fn new_game(&mut self) {
        self.tt.clear();
    }

    fn set_option(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name.eq_ignore_ascii_case("OwnBook") {
            let v = value.trim().to_ascii_lowercase();
            self.use_own_book = matches!(v.as_str(), "true" | "1" | "yes" | "on");
            return Ok(());
        }
        if name.eq_ignore_ascii_case("Hash") {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("invalid Hash value '{value}'"))?;
            self.hash_mb = parsed.max(1);
            self.tt = TranspositionTable::new_with_mb(self.hash_mb);
            return Ok(());
        }
        Ok(())
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        if self.use_own_book && params.depth.is_none() && game_state.ply < 20 {
            let mut rng = rng();
            if let Some(book_move) = self.opening_book.choose_weighted_move(game_state, &mut rng) {
                let mut out = EngineOutput::default();
                out.best_move = Some(book_move);
                out.info_lines
                    .push("info string opening book move".to_owned());
                return Ok(out);
            }
        }

        // Honor explicit UCI depth limits first; otherwise fall back to the
        // configured difficulty depth for this engine instance.
        let depth = params.depth.unwrap_or(self.default_depth).max(1);

        let result = match self.scorer_kind {
            IterativeScorerKind::Standard => iterative_deepening_search_with_tt(
                game_state,
                &self.move_generator,
                &self.standard_scorer,
                SearchConfig {
                    max_depth: depth,
                    movetime_ms: params.movetime_ms,
                },
                &mut self.tt,
            ),
            IterativeScorerKind::AlphaZero => iterative_deepening_search_with_tt(
                game_state,
                &self.move_generator,
                &self.alpha_zero_scorer,
                SearchConfig {
                    max_depth: depth,
                    movetime_ms: params.movetime_ms,
                },
                &mut self.tt,
            ),
        }
        .map_err(|e| e.to_string())?;

        let mut out = EngineOutput::default();
        let mut probe = game_state.clone();
        let legal =
            generate_legal_move_descriptions_in_place(&mut probe).map_err(|e| e.to_string())?;

        let mut chosen = result.best_move.or_else(|| legal.first().copied());
        if let Some(best) = chosen {
            let preferred = prefer_queen_promotion(best, &legal);
            if preferred != best {
                out.info_lines
                    .push("info string iterative_engine_v10 queen_promotion_preferred".to_owned());
            }
            chosen = Some(preferred);
        }
        out.best_move = chosen;
        out.info_lines.push(format!(
            "info depth {} score cp {} nodes {} time {} nps {}",
            result.reached_depth, result.best_score, result.nodes, result.elapsed_ms, result.nps
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v10 default_depth {}",
            self.default_depth
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v10 scorer {:?}",
            self.scorer_kind
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v10 used_depth {}",
            depth
        ));
        if let Some(ms) = params.movetime_ms {
            out.info_lines.push(format!(
                "info string iterative_engine_v10 movetime_ms {}",
                ms
            ));
        }
        out.info_lines.push(format!(
            "info string tt probes {} hits {} stores {} size_entries {}",
            result.tt_stats.probes,
            result.tt_stats.hits,
            result.tt_stats.stores,
            self.tt.len()
        ));

        let pv = principal_variation_from_tt(game_state, &mut self.tt, result.reached_depth);
        if !pv.moves.is_empty() {
            let mut pv_lan = Vec::with_capacity(pv.moves.len());
            let mut state = game_state.clone();
            for m in pv.moves {
                if let Ok(lan) = move_description_to_long_algebraic(m, &state) {
                    pv_lan.push(lan);
                } else {
                    break;
                }
                if let Ok(next) = crate::move_generation::legal_move_apply::apply_move(&state, m) {
                    state = next;
                } else {
                    break;
                }
            }
            if !pv_lan.is_empty() {
                out.info_lines.push(format!("info pv {}", pv_lan.join(" ")));
            }
        }

        Ok(out)
    }
}

fn prefer_queen_promotion(chosen: u64, legal_moves: &[u64]) -> u64 {
    let queen_promotions: Vec<u64> = legal_moves
        .iter()
        .copied()
        .filter(|mv| is_queen_promotion_move(*mv))
        .collect();
    if queen_promotions.is_empty() {
        return chosen;
    }

    if is_queen_promotion_move(chosen) {
        return chosen;
    }

    if is_any_promotion_move(chosen) {
        let from = move_from(chosen);
        let to = move_to(chosen);
        if let Some(same_square_queen) = queen_promotions
            .iter()
            .copied()
            .find(|m| move_from(*m) == from && move_to(*m) == to)
        {
            return same_square_queen;
        }
    }

    queen_promotions[0]
}

#[inline]
fn is_any_promotion_move(mv: u64) -> bool {
    piece_kind_from_code(move_promotion_piece_code(mv)).is_some()
}

#[inline]
fn is_queen_promotion_move(mv: u64) -> bool {
    matches!(
        piece_kind_from_code(move_promotion_piece_code(mv)),
        Some(crate::game_state::chess_types::PieceKind::Queen)
    )
}

impl From<IterativeScorerKind> for V3MaterialKind {
    fn from(value: IterativeScorerKind) -> Self {
        match value {
            IterativeScorerKind::Standard => V3MaterialKind::Standard,
            IterativeScorerKind::AlphaZero => V3MaterialKind::AlphaZero,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IterativeEngine;
    use crate::engines::engine_trait::{Engine, GoParams};
    use crate::game_state::game_state::GameState;

    #[test]
    fn iterative_engine_honors_go_depth_override() {
        let game = GameState::new_game();
        let mut engine = IterativeEngine::new(5);
        let params = GoParams {
            depth: Some(1),
            ..GoParams::default()
        };

        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let joined = out.info_lines.join("\n");

        assert!(
            joined.contains("info depth 1"),
            "expected depth-1 search info"
        );
        assert!(
            joined.contains("used_depth 1"),
            "expected used_depth=1 info"
        );
    }
}
