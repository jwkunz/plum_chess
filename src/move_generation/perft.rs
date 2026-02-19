//! Perft validation and benchmarking counters.
//!
//! Recursively explores legal move trees to verify correctness and collect
//! tactical event counts (captures, checks, promotions, etc.).

use std::sync::Arc;
use std::thread;

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::LegalMoveGenerator;
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

pub fn perft<G: MoveGenerator>(
    generator: &G,
    game_state: &GameState,
    depth: u8,
) -> MoveGenResult<PerftCounts> {
    perft_single_thread(generator, game_state, depth)
}

pub fn perft_legal(game_state: &GameState, depth: u8) -> MoveGenResult<PerftCounts> {
    let generator = LegalMoveGenerator;
    perft(&generator, game_state, depth)
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
        let (result, local) = handle.join().map_err(|_| {
            MoveGenerationError::InvalidState("perft worker thread panicked".to_owned())
        })?;
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
    use crate::game_state::game_state::GameState;
    use crate::move_generation::move_generator::MoveAnnotations;
    use crate::moves::move_descriptions::{
        pack_move_description, FLAG_CAPTURE, FLAG_CASTLING, FLAG_DOUBLE_PAWN_PUSH, FLAG_EN_PASSANT,
    };

    use super::*;

    struct MockMoveGenerator;

    impl MoveGenerator for MockMoveGenerator {
        fn generate_legal_moves(
            &self,
            game_state: &GameState,
        ) -> MoveGenResult<Vec<GeneratedMove>> {
            match game_state.ply {
                0 => Ok(vec![
                    next_move(
                        game_state,
                        12,
                        28,
                        PieceKind::Pawn,
                        None,
                        None,
                        FLAG_DOUBLE_PAWN_PUSH,
                        MoveAnnotations::default(),
                        1,
                    ),
                    next_move(
                        game_state,
                        4,
                        6,
                        PieceKind::King,
                        None,
                        None,
                        FLAG_CASTLING,
                        MoveAnnotations::default(),
                        2,
                    ),
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
                    next_move(
                        game_state,
                        6,
                        21,
                        PieceKind::King,
                        None,
                        None,
                        0,
                        MoveAnnotations::default(),
                        5,
                    ),
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

        let move_description = pack_move_description(
            from,
            to,
            moved_piece,
            captured_piece,
            promotion_piece,
            flags,
        );

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

    #[test]
    fn perft_start_position_nodes_depth_1_to_3() {
        let game = GameState::new_game();
        let expected = [20usize, 400, 8902];

        for (idx, target_nodes) in expected.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let counts = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(
                counts.nodes, *target_nodes,
                "node mismatch at depth {depth}"
            );
        }
    }

    #[test]
    fn perft_position_1_depth_1_to_4_full_counts() {
        let game = GameState::new_game();
        let results = [
            PerftCounts {
                nodes: 20,
                captures: 0,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 0,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 400,
                captures: 0,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 0,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 8902,
                captures: 34,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 12,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 197281,
                captures: 1576,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 469,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 8,
            },
        ];

        for (idx, target) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count, *target, "mismatch at depth {depth}");
        }
    }

    #[test]
    fn perft_position_2_depth_1_to_3_full_counts() {
        let game = GameState::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0",
        )
        .expect("FEN should parse");
        let results = [
            PerftCounts {
                nodes: 48,
                captures: 8,
                en_passant: 0,
                castles: 2,
                promotions: 0,
                checks: 0,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 2039,
                captures: 351,
                en_passant: 1,
                castles: 91,
                promotions: 0,
                checks: 3,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 97862,
                captures: 17102,
                en_passant: 45,
                castles: 3162,
                promotions: 0,
                checks: 993,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 1,
            },
        ];

        for (idx, target) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count, *target, "mismatch at depth {depth}");
        }
    }

    #[test]
    fn perft_position_3_depth_1_to_5_full_counts() {
        let game = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1")
            .expect("FEN should parse");
        let results = [
            PerftCounts {
                nodes: 14,
                captures: 1,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 2,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 191,
                captures: 14,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 10,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 2812,
                captures: 209,
                en_passant: 2,
                castles: 0,
                promotions: 0,
                checks: 267,
                discovery_checks: 3,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 43238,
                captures: 3348,
                en_passant: 123,
                castles: 0,
                promotions: 0,
                checks: 1680,
                discovery_checks: 106,
                double_checks: 0,
                checkmates: 17,
            },
            PerftCounts {
                nodes: 674624,
                captures: 52051,
                en_passant: 1165,
                castles: 0,
                promotions: 0,
                checks: 52950,
                discovery_checks: 1292,
                double_checks: 3,
                checkmates: 0,
            },
        ];

        for (idx, target) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count, *target, "mismatch at depth {depth}");
        }
    }

    #[test]
    fn perft_position_4_depth_1_to_4_full_counts() {
        let game =
            GameState::from_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1")
                .expect("FEN should parse");
        let results = [
            PerftCounts {
                nodes: 6,
                captures: 0,
                en_passant: 0,
                castles: 0,
                promotions: 0,
                checks: 0,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 264,
                captures: 87,
                en_passant: 0,
                castles: 6,
                promotions: 48,
                checks: 10,
                discovery_checks: 0,
                double_checks: 0,
                checkmates: 0,
            },
            PerftCounts {
                nodes: 9467,
                captures: 1021,
                en_passant: 4,
                castles: 0,
                promotions: 120,
                checks: 38,
                discovery_checks: 2,
                double_checks: 0,
                checkmates: 22,
            },
            PerftCounts {
                nodes: 422333,
                captures: 131393,
                en_passant: 0,
                castles: 7795,
                promotions: 60032,
                checks: 15492,
                discovery_checks: 19,
                double_checks: 0,
                checkmates: 5,
            },
        ];

        for (idx, target) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count, *target, "mismatch at depth {depth}");
        }
    }

    #[test]
    fn perft_position_5_nodes_depth_1_to_4() {
        let game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
            .expect("FEN should parse");
        let results = [44usize, 1486, 62379, 2103487];

        for (idx, target_nodes) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count.nodes, *target_nodes, "node mismatch at depth {depth}");
        }
    }

    #[test]
    fn perft_position_6_nodes_depth_1_to_4() {
        let game = GameState::from_fen(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        )
        .expect("FEN should parse");
        let results = [46usize, 2079, 89890, 3894594];

        for (idx, target_nodes) in results.iter().enumerate() {
            let depth = (idx + 1) as u8;
            let count = perft_legal(&game, depth).expect("perft should run");
            assert_eq!(count.nodes, *target_nodes, "node mismatch at depth {depth}");
        }
    }
}
