//! Iterative-deepening material-search engine (V15).
//!
//! Wraps the core negamax alpha-beta search with fixed-depth configuration and
//! material scoring for deterministic stronger difficulty levels.
//!
//! V15 marker:
//! - Successor to V14 with contempt + draw-avoidance when clearly winning.
//! - Supports legacy `1/20` and adaptive budget allocation.

use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::engines::time_management::{resolve_go_params, TimeManagementStrategy};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_checks::is_king_in_check;
use crate::move_generation::legal_move_generator::{
    generate_legal_move_descriptions_in_place, FastLegalMoveGenerator,
};
use crate::moves::move_descriptions::{
    move_from, move_promotion_piece_code, move_to, piece_kind_from_code,
};
use crate::search::board_scoring::{BoardScorer, EndgameTaperedScorerV14, V3MaterialKind};
use crate::search::iterative_deepening_v15::{
    iterative_deepening_search_with_tt, principal_variation_from_tt, SearchConfig,
};
use crate::search::transposition_table_v11::TranspositionTable;
use crate::tables::opening_book::OpeningBook;
use crate::utils::long_algebraic::move_description_to_long_algebraic;
use rand::rng;
use std::sync::{atomic::AtomicBool, Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterativeScorerKind {
    Standard,
    AlphaZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoControlMode {
    MoveTime,
    Nodes,
    Mate,
    ClocksOrDepth,
}

pub struct IterativeEngine {
    default_depth: u8,
    move_generator: FastLegalMoveGenerator,
    standard_scorer: EndgameTaperedScorerV14,
    alpha_zero_scorer: EndgameTaperedScorerV14,
    scorer_kind: IterativeScorerKind,
    opening_book: OpeningBook,
    use_own_book: bool,
    tt: TranspositionTable,
    hash_mb: usize,
    multipv: usize,
    time_strategy: TimeManagementStrategy,
    stop_signal: Option<Arc<AtomicBool>>,
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
            standard_scorer: EndgameTaperedScorerV14::standard(),
            alpha_zero_scorer: EndgameTaperedScorerV14::alpha_zero(),
            scorer_kind,
            opening_book: OpeningBook::load_default(),
            use_own_book: true,
            tt: TranspositionTable::new_with_mb(hash_mb),
            hash_mb,
            multipv: 1,
            time_strategy: TimeManagementStrategy::AdaptiveV13,
            stop_signal: None,
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
        if name.eq_ignore_ascii_case("TimeStrategy") {
            let v = value.trim().to_ascii_lowercase();
            self.time_strategy = match v.as_str() {
                "adaptive" | "v13" => TimeManagementStrategy::AdaptiveV13,
                "fraction20" | "legacy" | "simple" => TimeManagementStrategy::Fraction20,
                _ => return Err(format!("invalid TimeStrategy value '{value}'")),
            };
            return Ok(());
        }
        if name.eq_ignore_ascii_case("MultiPV") {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("invalid MultiPV value '{value}'"))?;
            self.multipv = parsed.clamp(1, 32);
            return Ok(());
        }
        Ok(())
    }

    fn set_stop_signal(&mut self, stop_signal: Option<Arc<AtomicBool>>) {
        self.stop_signal = stop_signal;
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        let control_mode = if params.movetime_ms.is_some() {
            GoControlMode::MoveTime
        } else if params.nodes.is_some() {
            GoControlMode::Nodes
        } else if params.mate.is_some() {
            GoControlMode::Mate
        } else {
            GoControlMode::ClocksOrDepth
        };

        let mate_mode = if control_mode == GoControlMode::Mate {
            params.mate
        } else {
            None
        };
        let node_cap = if control_mode == GoControlMode::Nodes {
            params.nodes
        } else {
            None
        };
        let mut effective_params = resolve_go_params(game_state, params, self.time_strategy);
        if matches!(control_mode, GoControlMode::Mate | GoControlMode::Nodes)
            && params.movetime_ms.is_none()
        {
            // In mate/nodes modes, prioritize these explicit controls over adaptive clock slicing.
            effective_params.movetime_ms = None;
        }
        let requested_searchmoves = params.searchmoves.as_deref();
        if self.use_own_book
            && mate_mode.is_none()
            && effective_params.depth.is_none()
            && game_state.ply < 20
        {
            let mut rng = rng();
            if let Some(book_move) = self.opening_book.choose_weighted_move(game_state, &mut rng) {
                if let Some(allowed) = requested_searchmoves {
                    if !allowed.contains(&book_move) {
                        // Continue into full search if the book move is outside
                        // the restricted root set from `go searchmoves`.
                    } else {
                        let mut out = EngineOutput::default();
                        out.best_move = Some(book_move);
                        out.info_lines
                            .push("info string opening book move".to_owned());
                        return Ok(out);
                    }
                } else {
                    let mut out = EngineOutput::default();
                    out.best_move = Some(book_move);
                    out.info_lines
                        .push("info string opening book move".to_owned());
                    return Ok(out);
                }
            }
        }

        // Honor explicit UCI depth limits first; otherwise fall back to the
        // configured difficulty depth for this engine instance.
        let mate_depth_target = mate_mode.map(|m| m.saturating_mul(2).saturating_add(1).max(1));
        let depth = effective_params
            .depth
            .unwrap_or(self.default_depth)
            .max(mate_depth_target.unwrap_or(1))
            .max(1);

        let result = match self.scorer_kind {
            IterativeScorerKind::Standard => iterative_deepening_search_with_tt(
                game_state,
                &self.move_generator,
                &self.standard_scorer,
                SearchConfig {
                    max_depth: depth,
                    movetime_ms: effective_params.movetime_ms,
                    max_nodes: node_cap,
                    stop_flag: self.stop_signal.clone(),
                },
                &mut self.tt,
            ),
            IterativeScorerKind::AlphaZero => iterative_deepening_search_with_tt(
                game_state,
                &self.move_generator,
                &self.alpha_zero_scorer,
                SearchConfig {
                    max_depth: depth,
                    movetime_ms: effective_params.movetime_ms,
                    max_nodes: node_cap,
                    stop_flag: self.stop_signal.clone(),
                },
                &mut self.tt,
            ),
        }
        .map_err(|e| e.to_string())?;

        let mut out = EngineOutput::default();
        let mut probe = game_state.clone();
        let legal =
            generate_legal_move_descriptions_in_place(&mut probe).map_err(|e| e.to_string())?;
        let root_legal: Vec<u64> = if let Some(allowed) = requested_searchmoves {
            legal
                .iter()
                .copied()
                .filter(|mv| allowed.contains(mv))
                .collect()
        } else {
            legal.clone()
        };
        if root_legal.is_empty() {
            out.info_lines.push(
                "info string iterative_engine_v15 no legal root move in requested searchmoves"
                    .to_owned(),
            );
            return Ok(out);
        }

        let mut chosen = result
            .best_move
            .filter(|mv| root_legal.contains(mv))
            .or_else(|| root_legal.first().copied());

        if let Some(mate_one) = find_mate_in_one(game_state, &root_legal) {
            chosen = Some(mate_one);
            if mate_mode.is_some() {
                out.info_lines.push(
                    "info string iterative_engine_v15 mate_mode immediate_mate_selected".to_owned(),
                );
            } else {
                out.info_lines.push(
                    "info string iterative_engine_v15 mate_score_shaping immediate_mate_selected"
                        .to_owned(),
                );
            }
        }

        if let Some(best) = chosen {
            let preferred = prefer_queen_promotion(best, &root_legal);
            if preferred != best {
                out.info_lines
                    .push("info string iterative_engine_v15 queen_promotion_preferred".to_owned());
            }
            chosen = Some(preferred);
        }
        out.best_move = chosen;

        if self.multipv > 1 {
            let multipv_lines = self.build_multipv_lines(game_state, &root_legal, depth);
            out.info_lines.extend(multipv_lines);
        }

        out.info_lines.push(format!(
            "info depth {} score cp {} nodes {} time {} nps {}",
            result.reached_depth, result.best_score, result.nodes, result.elapsed_ms, result.nps
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 default_depth {}",
            self.default_depth
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 scorer {:?}",
            self.scorer_kind
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 used_depth {}",
            depth
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 control_mode {:?}",
            control_mode
        ));
        if let Some(mate) = mate_mode {
            out.info_lines.push(format!(
                "info string iterative_engine_v15 mate_mode plies_target {}",
                mate.saturating_mul(2).saturating_add(1)
            ));
        }
        if let Some(ms) = effective_params.movetime_ms {
            out.info_lines.push(format!(
                "info string iterative_engine_v15 movetime_ms {}",
                ms
            ));
        }
        out.info_lines.push(format!(
            "info string iterative_engine_v15 time_strategy {:?}",
            self.time_strategy
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 multipv {}",
            self.multipv
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 go_raw movetime={:?} wtime={:?} btime={:?} winc={:?} binc={:?}",
            params.movetime_ms, params.wtime_ms, params.btime_ms, params.winc_ms, params.binc_ms
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 go_modes nodes={:?} mate={:?} ponder={} infinite={}",
            params.nodes, params.mate, params.ponder, params.infinite
        ));
        if let Some(node_cap) = node_cap {
            out.info_lines.push(format!(
                "info string iterative_engine_v15 node_cap {}",
                node_cap
            ));
        }
        if params.ponder {
            out.info_lines.push(
                "info string iterative_engine_v15 note ponder mode parsed; search remains synchronous"
                    .to_owned(),
            );
        }
        if params.infinite {
            out.info_lines.push(
                "info string iterative_engine_v15 note infinite parsed; bounded iterative search is used in synchronous mode"
                    .to_owned(),
            );
        }
        out.info_lines.push(format!(
            "info string iterative_engine_v15 go_resolved movetime={:?}",
            effective_params.movetime_ms
        ));
        out.info_lines.push(format!(
            "info string iterative_engine_v15 movetime_source {}",
            if control_mode == GoControlMode::MoveTime {
                "explicit"
            } else if control_mode == GoControlMode::Mate {
                "mate_mode_unbounded"
            } else if control_mode == GoControlMode::Nodes {
                "node_mode_unbounded"
            } else {
                "strategy"
            }
        ));
        if control_mode == GoControlMode::MoveTime
            && (params.nodes.is_some() || params.mate.is_some())
        {
            out.info_lines.push(
                "info string iterative_engine_v15 precedence movetime overrides nodes/mate"
                    .to_owned(),
            );
        } else if control_mode == GoControlMode::Nodes && params.mate.is_some() {
            out.info_lines.push(
                "info string iterative_engine_v15 precedence nodes overrides mate".to_owned(),
            );
        }
        out.info_lines.push(format!(
            "info string tt probes {} hits {} stores {} size_entries {}",
            result.tt_stats.probes,
            result.tt_stats.hits,
            result.tt_stats.stores,
            self.tt.len()
        ));

        let pv = principal_variation_from_tt(game_state, &mut self.tt, result.reached_depth);
        if pv.moves.len() >= 2 {
            out.ponder_move = Some(pv.moves[1]);
        }
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

impl IterativeEngine {
    fn build_multipv_lines(&self, game_state: &GameState, root_legal: &[u64], depth: u8) -> Vec<String> {
        let scorer = match self.scorer_kind {
            IterativeScorerKind::Standard => &self.standard_scorer as &dyn BoardScorer,
            IterativeScorerKind::AlphaZero => &self.alpha_zero_scorer as &dyn BoardScorer,
        };

        let mut ranked: Vec<(u64, i32)> = root_legal
            .iter()
            .copied()
            .map(|mv| (mv, score_root_candidate(game_state, mv, scorer)))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));

        let n = ranked.len().min(self.multipv);
        let mut lines = Vec::with_capacity(n);
        for (idx, (mv, cp)) in ranked.into_iter().take(n).enumerate() {
            if let Ok(lan) = move_description_to_long_algebraic(mv, game_state) {
                lines.push(format!(
                    "info depth {} multipv {} score cp {} pv {}",
                    depth,
                    idx + 1,
                    cp,
                    lan
                ));
            }
        }
        lines
    }
}

fn score_root_candidate(game_state: &GameState, mv: u64, scorer: &dyn BoardScorer) -> i32 {
    let Ok(next) = apply_move(game_state, mv) else {
        return i32::MIN / 4;
    };
    let mut probe = next.clone();
    let Ok(replies) = generate_legal_move_descriptions_in_place(&mut probe) else {
        return -scorer.score(&next);
    };
    if replies.is_empty() && is_king_in_check(&next, next.side_to_move) {
        return 29_500;
    }
    -scorer.score(&next)
}

fn find_mate_in_one(game_state: &GameState, legal_moves: &[u64]) -> Option<u64> {
    for mv in legal_moves {
        let Ok(next) = apply_move(game_state, *mv) else {
            continue;
        };
        let mut probe = next.clone();
        let Ok(replies) = generate_legal_move_descriptions_in_place(&mut probe) else {
            continue;
        };
        if replies.is_empty() && is_king_in_check(&next, next.side_to_move) {
            return Some(*mv);
        }
    }
    None
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
    use crate::move_generation::legal_move_apply::apply_move;
    use crate::move_generation::legal_move_checks::is_king_in_check;
    use crate::move_generation::legal_move_generator::generate_legal_move_descriptions_in_place;

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

    #[test]
    fn iterative_engine_mate_mode_prefers_checkmate() {
        let game =
            GameState::from_fen("6k1/5Q2/6K1/8/8/8/8/8 w - - 0 1").expect("FEN should parse");
        let mut engine = IterativeEngine::new(2);
        let params = GoParams {
            mate: Some(1),
            ..GoParams::default()
        };

        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let best = out.best_move.expect("best move should exist");
        let next = apply_move(&game, best).expect("best move should apply");
        let mut probe = next.clone();
        let replies =
            generate_legal_move_descriptions_in_place(&mut probe).expect("legal moves should run");
        assert!(replies.is_empty(), "mate mode should choose a mating move");
        assert!(
            is_king_in_check(&next, next.side_to_move),
            "chosen move should deliver checkmate"
        );
    }

    #[test]
    fn iterative_engine_precedence_movetime_overrides_nodes_and_mate() {
        let game = GameState::new_game();
        let mut engine = IterativeEngine::new(3);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        let params = GoParams {
            movetime_ms: Some(20),
            nodes: Some(10),
            mate: Some(2),
            ..GoParams::default()
        };
        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let joined = out.info_lines.join("\n");
        assert!(joined.contains("control_mode MoveTime"));
        assert!(joined.contains("precedence movetime overrides nodes/mate"));
        assert!(!joined.contains("node_cap 10"));
        assert!(!joined.contains("mate_mode plies_target"));
    }

    #[test]
    fn iterative_engine_precedence_nodes_overrides_mate() {
        let game = GameState::new_game();
        let mut engine = IterativeEngine::new(4);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        let params = GoParams {
            nodes: Some(200),
            mate: Some(3),
            ..GoParams::default()
        };
        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let joined = out.info_lines.join("\n");
        assert!(joined.contains("control_mode Nodes"));
        assert!(joined.contains("precedence nodes overrides mate"));
        assert!(joined.contains("node_cap 200"));
        assert!(!joined.contains("mate_mode plies_target"));
    }

    #[test]
    fn iterative_engine_emits_multipv_lines_when_enabled() {
        let game = GameState::new_game();
        let mut engine = IterativeEngine::new(2);
        engine
            .set_option("OwnBook", "false")
            .expect("setoption should work");
        engine
            .set_option("MultiPV", "3")
            .expect("multipv should parse");
        let params = GoParams {
            depth: Some(1),
            ..GoParams::default()
        };
        let out = engine
            .choose_move(&game, &params)
            .expect("engine should choose a move");
        let joined = out.info_lines.join("\n");
        assert!(joined.contains("multipv 1"));
        assert!(joined.contains("multipv 2"));
    }
}
