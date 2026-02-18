//! Full legal move generation pipeline.
//!
//! Orchestrates piece-wise pseudo-legal generation, applies candidate moves,
//! filters illegal self-check outcomes, and annotates checking move metadata.

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::move_generation::legal_move_checks::{
    attackers_to_square, is_king_in_check, king_square,
};
use crate::move_generation::legal_moves_bishop::generate_bishop_moves;
use crate::move_generation::legal_moves_king::generate_king_moves;
use crate::move_generation::legal_moves_knight::generate_knight_moves;
use crate::move_generation::legal_moves_pawn::generate_pawn_moves;
use crate::move_generation::legal_moves_queen::generate_queen_moves;
use crate::move_generation::legal_moves_rook::generate_rook_moves;
use crate::move_generation::move_generator::{
    GeneratedMove, MoveAnnotations, MoveGenResult, MoveGenerationError, MoveGenerator,
};
use crate::moves::move_descriptions::{
    move_from, move_moved_piece_code, move_promotion_piece_code, move_to, piece_kind_from_code,
};

pub struct LegalMoveGenerator;
pub struct FastLegalMoveGenerator;

impl MoveGenerator for LegalMoveGenerator {
    fn generate_legal_moves(&self, game_state: &GameState) -> MoveGenResult<Vec<GeneratedMove>> {
        self.generate_legal_moves_internal(game_state, true)
    }
}

impl MoveGenerator for FastLegalMoveGenerator {
    fn generate_legal_moves(&self, game_state: &GameState) -> MoveGenResult<Vec<GeneratedMove>> {
        LegalMoveGenerator.generate_legal_moves_internal(game_state, false)
    }
}

impl LegalMoveGenerator {
    fn generate_legal_moves_internal(
        &self,
        game_state: &GameState,
        annotate: bool,
    ) -> MoveGenResult<Vec<GeneratedMove>> {
        let mut pseudo = Vec::<u64>::with_capacity(128);

        generate_pawn_moves(game_state, &mut pseudo);
        generate_knight_moves(game_state, &mut pseudo);
        generate_bishop_moves(game_state, &mut pseudo);
        generate_rook_moves(game_state, &mut pseudo);
        generate_queen_moves(game_state, &mut pseudo);
        generate_king_moves(game_state, &mut pseudo);

        let mut legal = Vec::<GeneratedMove>::with_capacity(pseudo.len());
        for mv in pseudo {
            let next = apply_move(game_state, mv).map_err(|x| {
                MoveGenerationError::InvalidState(format!("apply_move failed: {x}"))
            })?;

            // Illegal if own king is in check after move.
            if is_king_in_check(&next, game_state.side_to_move) {
                continue;
            }

            let annotations = if annotate {
                classify_move_annotations(self, game_state, mv, &next)?
            } else {
                MoveAnnotations::default()
            };

            legal.push(GeneratedMove {
                move_description: mv,
                game_after_move: next,
                annotations,
            });
        }

        Ok(legal)
    }
}

fn classify_move_annotations(
    generator: &LegalMoveGenerator,
    prev: &GameState,
    move_description: u64,
    next: &GameState,
) -> MoveGenResult<MoveAnnotations> {
    let Some(defender_king_sq) = king_square(next, next.side_to_move) else {
        return Ok(MoveAnnotations::default());
    };

    let attacker_color = prev.side_to_move;
    let checkers = attackers_to_square(next, defender_king_sq, attacker_color);
    if checkers.is_empty() {
        return Ok(MoveAnnotations::default());
    }

    let to = move_to(move_description);
    let moved_piece = piece_kind_from_code(move_moved_piece_code(move_description))
        .ok_or_else(|| MoveGenerationError::InvalidState("invalid moved piece code".to_owned()))?;
    let moved_piece_after =
        piece_kind_from_code(move_promotion_piece_code(move_description)).unwrap_or(moved_piece);

    let moved_piece_is_checker = checkers
        .iter()
        .any(|(sq, piece)| *sq == to && *piece == moved_piece_after);

    let is_double_check = checkers.len() >= 2;
    let is_discovery_check = if !is_double_check && !moved_piece_is_checker {
        let checker = checkers[0];
        let from = move_from(move_description);
        is_discovered_line_check(from, checker, defender_king_sq)
    } else {
        false
    };

    let reply_count = generator.generate_legal_moves_internal(next, false)?.len();

    Ok(MoveAnnotations {
        gives_check: true,
        is_discovery_check,
        is_double_check,
        is_checkmate: reply_count == 0,
    })
}

fn is_discovered_line_check(
    from: u8,
    checker: (u8, crate::game_state::chess_types::PieceKind),
    king_sq: u8,
) -> bool {
    use crate::game_state::chess_types::PieceKind;
    let (checker_sq, checker_piece) = checker;
    match checker_piece {
        PieceKind::Bishop | PieceKind::Rook | PieceKind::Queen => {}
        _ => return false,
    }

    is_square_between(from, checker_sq, king_sq)
}

fn is_square_between(mid: u8, a: u8, b: u8) -> bool {
    if mid == a || mid == b {
        return false;
    }

    let af = (a % 8) as i8;
    let ar = (a / 8) as i8;
    let bf = (b % 8) as i8;
    let br = (b / 8) as i8;
    let mf = (mid % 8) as i8;
    let mr = (mid / 8) as i8;

    let df = bf - af;
    let dr = br - ar;

    let step_f = df.signum();
    let step_r = dr.signum();

    // Must be aligned on rank/file/diagonal.
    let aligned = df == 0 || dr == 0 || df.abs() == dr.abs();
    if !aligned {
        return false;
    }

    let mut f = af + step_f;
    let mut r = ar + step_r;
    while f != bf || r != br {
        if f == mf && r == mr {
            return true;
        }
        f += step_f;
        r += step_r;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{FastLegalMoveGenerator, LegalMoveGenerator};
    use crate::game_state::game_state::GameState;
    use crate::move_generation::move_generator::MoveGenerator;

    #[test]
    fn fast_generator_matches_legal_move_count_on_startpos() {
        let game = GameState::new_game();
        let annotated = LegalMoveGenerator
            .generate_legal_moves(&game)
            .expect("annotated move generation should succeed");
        let fast = FastLegalMoveGenerator
            .generate_legal_moves(&game)
            .expect("fast move generation should succeed");
        assert_eq!(annotated.len(), fast.len());
        assert_eq!(fast.len(), 20);
    }
}
