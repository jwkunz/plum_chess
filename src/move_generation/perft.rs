use std::sync::Arc;
use std::thread;

use crate::game_state::game_state::GameState;
use crate::move_generation::move_generator::{
    GeneratedMove, MoveGenResult, MoveGenerationError, MoveGenerator,
};
use crate::moves::move_descriptions::{
    move_promotion_piece_code, FLAG_CAPTURE, FLAG_CASTLING, FLAG_EN_PASSANT, NO_PIECE_CODE,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PerftCounts {
    pub nodes: usize,
    pub captures: usize,
    pub en_passant: usize,
    pub castles: usize,
    pub promotions: usize,
    pub checks: usize,
    pub discovery_checks: usize,
    pub double_checks: usize,
    pub checkmates: usize,
}

impl PerftCounts {
    fn merge(&mut self, rhs: PerftCounts) {
        self.nodes += rhs.nodes;
        self.captures += rhs.captures;
        self.en_passant += rhs.en_passant;
        self.castles += rhs.castles;
        self.promotions += rhs.promotions;
        self.checks += rhs.checks;
        self.discovery_checks += rhs.discovery_checks;
        self.double_checks += rhs.double_checks;
        self.checkmates += rhs.checkmates;
    }
}

pub fn perft<G: MoveGenerator>(generator: &G, game_state: &GameState, depth: u8) -> MoveGenResult<PerftCounts> {
    perft_single_thread(generator, game_state, depth)
}

pub fn perft_single_thread<G: MoveGenerator>(
    generator: &G,
    game_state: &GameState,
    depth: u8,
) -> MoveGenResult<PerftCounts> {
    if depth == 0 {
        return Ok(PerftCounts {
            nodes: 1,
            ..PerftCounts::default()
        });
    }

    let root_moves = generator.generate_legal_moves(game_state)?;
    let mut total = PerftCounts::default();

    for mv in root_moves {
        perft_recurse(generator, &mv, depth, 1, &mut total)?;
    }

    Ok(total)
}

pub fn perft_multi_threaded(
    generator: Arc<dyn MoveGenerator>,
    game_state: &GameState,
    depth: u8,
) -> MoveGenResult<PerftCounts> {
    if depth == 0 {
        return Ok(PerftCounts {
            nodes: 1,
            ..PerftCounts::default()
        });
    }

    let root_moves = generator.generate_legal_moves(game_state)?;
    let mut handles = Vec::with_capacity(root_moves.len());

    for mv in root_moves {
        let generator_ref = Arc::clone(&generator);
        handles.push(thread::spawn(move || {
            let mut local = PerftCounts::default();
            let result = perft_recurse(generator_ref.as_ref(), &mv, depth, 1, &mut local);
            (result, local)
        }));
    }

    let mut total = PerftCounts::default();
    for handle in handles {
        let (result, local) = handle
            .join()
            .map_err(|_| MoveGenerationError::InvalidState("perft worker thread panicked".to_owned()))?;
        result?;
        total.merge(local);
    }

    Ok(total)
}

fn perft_recurse(
    generator: &dyn MoveGenerator,
    mv: &GeneratedMove,
    search_depth: u8,
    current_depth: u8,
    counts: &mut PerftCounts,
) -> MoveGenResult<()> {
    if current_depth == search_depth {
        counts.nodes += 1;

        if (mv.move_description & FLAG_CAPTURE) != 0 {
            counts.captures += 1;
        }
        if (mv.move_description & FLAG_EN_PASSANT) != 0 {
            counts.en_passant += 1;
        }
        if (mv.move_description & FLAG_CASTLING) != 0 {
            counts.castles += 1;
        }
        if move_promotion_piece_code(mv.move_description) != NO_PIECE_CODE {
            counts.promotions += 1;
        }

        if mv.annotations.gives_check {
            counts.checks += 1;
        }
        if mv.annotations.is_discovery_check {
            counts.discovery_checks += 1;
        }
        if mv.annotations.is_double_check {
            counts.double_checks += 1;
        }
        if mv.annotations.is_checkmate {
            counts.checkmates += 1;
        }

        return Ok(());
    }

    let moves = generator.generate_legal_moves(&mv.game_after_move)?;
    for child in moves {
        perft_recurse(generator, &child, search_depth, current_depth + 1, counts)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::game_state::chess_types::{Color, PieceKind};
    use crate::move_generation::move_generator::MoveAnnotations;
    use crate::moves::move_descriptions::{
        pack_move_description, FLAG_CAPTURE, FLAG_CASTLING, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT,
    };

    use super::*;

    struct MockMoveGenerator;

    impl MoveGenerator for MockMoveGenerator {
        fn generate_legal_moves(&self, game_state: &GameState) -> MoveGenResult<Vec<GeneratedMove>> {
            match game_state.ply {
                0 => Ok(vec![
                    next_move(game_state, 12, 28, PieceKind::Pawn, None, None, FLAG_DOUBLE_PAWN_PUSH, MoveAnnotations::default(), 1),
                    next_move(game_state, 4, 6, PieceKind::King, None, None, FLAG_CASTLING, MoveAnnotations::default(), 2),
                ]),
                1 if game_state.halfmove_clock == 1 => Ok(vec![
                    next_move(
                        game_state,
                        28,
                        35,
                        PieceKind::Pawn,
                        Some(PieceKind::Pawn),
                        None,
                        FLAG_CAPTURE,
                        MoveAnnotations {
                            gives_check: true,
                            is_discovery_check: true,
                            is_double_check: false,
                            is_checkmate: false,
                        },
                        3,
                    ),
                    next_move(
                        game_state,
                        28,
                        35,
                        PieceKind::Pawn,
                        Some(PieceKind::Pawn),
                        None,
                        FLAG_CAPTURE | FLAG_EN_PASSANT,
                        MoveAnnotations {
                            gives_check: true,
                            is_discovery_check: false,
                            is_double_check: true,
                            is_checkmate: false,
                        },
                        4,
                    ),
                ]),
                1 if game_state.halfmove_clock == 2 => Ok(vec![
                    next_move(game_state, 6, 21, PieceKind::King, None, None, 0, MoveAnnotations::default(), 5),
                    next_move(
                        game_state,
                        48,
                        56,
                        PieceKind::Pawn,
                        None,
                        Some(PieceKind::Queen),
                        0,
                        MoveAnnotations {
                            gives_check: true,
                            is_discovery_check: false,
                            is_double_check: false,
                            is_checkmate: true,
                        },
                        6,
                    ),
                ]),
                _ => Ok(Vec::new()),
            }
        }
    }

    fn next_move(
        game_state: &GameState,
        from: u8,
        to: u8,
        moved_piece: PieceKind,
        captured_piece: Option<PieceKind>,
        promotion_piece: Option<PieceKind>,
        flags: u64,
        annotations: MoveAnnotations,
        next_halfmove: u16,
    ) -> GeneratedMove {
        let mut game_after_move = game_state.clone();
        game_after_move.ply += 1;
        game_after_move.side_to_move = match game_after_move.side_to_move {
            Color::Light => Color::Dark,
            Color::Dark => Color::Light,
        };
        game_after_move.halfmove_clock = next_halfmove;

        let move_description =
            pack_move_description(from, to, moved_piece, captured_piece, promotion_piece, flags);

        GeneratedMove {
            move_description,
            game_after_move,
            annotations,
        }
    }

    #[test]
    fn perft_depth_zero_counts_single_node() {
        let generator = MockMoveGenerator;
        let game = GameState::new_empty();

        let counts = perft(&generator, &game, 0).expect("perft should run");
        assert_eq!(
            counts,
            PerftCounts {
                nodes: 1,
                ..PerftCounts::default()
            }
        );
    }

    #[test]
    fn perft_depth_two_aggregates_leaf_metrics() {
        let generator = MockMoveGenerator;
        let game = GameState::new_empty();

        let counts = perft(&generator, &game, 2).expect("perft should run");

        assert_eq!(
            counts,
            PerftCounts {
                nodes: 4,
                captures: 2,
                en_passant: 1,
                castles: 0,
                promotions: 1,
                checks: 3,
                discovery_checks: 1,
                double_checks: 1,
                checkmates: 1,
            }
        );
    }
}
