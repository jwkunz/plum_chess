use crate::game_state::{chess_types::*, game_state::GameState};
use crate::moves::bishop_moves::bishop_attacks;
use crate::moves::king_moves::king_attacks;
use crate::moves::knight_moves::knight_attacks;
use crate::moves::pawn_moves::pawn_attacks;
use crate::moves::rook_moves::rook_attacks;

#[inline]
pub fn king_square(game_state: &GameState, color: Color) -> Option<Square> {
    let kings = game_state.pieces[color.index()][PieceKind::King.index()];
    if kings == 0 {
        None
    } else {
        Some(kings.trailing_zeros() as Square)
    }
}

#[inline]
pub fn is_king_in_check(game_state: &GameState, color: Color) -> bool {
    let Some(king_sq) = king_square(game_state, color) else {
        return false;
    };
    is_square_attacked(game_state, king_sq, color.opposite())
}

pub fn is_square_attacked(game_state: &GameState, square: Square, attacker_color: Color) -> bool {
    let target_mask = 1u64 << square;

    let attacker_pawns = game_state.pieces[attacker_color.index()][PieceKind::Pawn.index()];
    let mut pawns = attacker_pawns;
    while pawns != 0 {
        let from = pawns.trailing_zeros() as Square;
        if pawn_attacks(attacker_color, from) & target_mask != 0 {
            return true;
        }
        pawns &= pawns - 1;
    }

    let attacker_knights = game_state.pieces[attacker_color.index()][PieceKind::Knight.index()];
    if knight_attacks(square) & attacker_knights != 0 {
        return true;
    }

    let attacker_kings = game_state.pieces[attacker_color.index()][PieceKind::King.index()];
    if king_attacks(square) & attacker_kings != 0 {
        return true;
    }

    let bishops_queens = game_state.pieces[attacker_color.index()][PieceKind::Bishop.index()]
        | game_state.pieces[attacker_color.index()][PieceKind::Queen.index()];
    if bishop_attacks(square, game_state.occupancy_all) & bishops_queens != 0 {
        return true;
    }

    let rooks_queens = game_state.pieces[attacker_color.index()][PieceKind::Rook.index()]
        | game_state.pieces[attacker_color.index()][PieceKind::Queen.index()];
    if rook_attacks(square, game_state.occupancy_all) & rooks_queens != 0 {
        return true;
    }

    false
}

pub fn attackers_to_square(
    game_state: &GameState,
    square: Square,
    attacker_color: Color,
) -> Vec<(Square, PieceKind)> {
    let target_mask = 1u64 << square;
    let mut attackers = Vec::<(Square, PieceKind)>::new();

    let mut pawns = game_state.pieces[attacker_color.index()][PieceKind::Pawn.index()];
    while pawns != 0 {
        let from = pawns.trailing_zeros() as Square;
        if pawn_attacks(attacker_color, from) & target_mask != 0 {
            attackers.push((from, PieceKind::Pawn));
        }
        pawns &= pawns - 1;
    }

    let mut knights = game_state.pieces[attacker_color.index()][PieceKind::Knight.index()];
    while knights != 0 {
        let from = knights.trailing_zeros() as Square;
        if knight_attacks(from) & target_mask != 0 {
            attackers.push((from, PieceKind::Knight));
        }
        knights &= knights - 1;
    }

    let mut bishops = game_state.pieces[attacker_color.index()][PieceKind::Bishop.index()];
    while bishops != 0 {
        let from = bishops.trailing_zeros() as Square;
        if bishop_attacks(from, game_state.occupancy_all) & target_mask != 0 {
            attackers.push((from, PieceKind::Bishop));
        }
        bishops &= bishops - 1;
    }

    let mut rooks = game_state.pieces[attacker_color.index()][PieceKind::Rook.index()];
    while rooks != 0 {
        let from = rooks.trailing_zeros() as Square;
        if rook_attacks(from, game_state.occupancy_all) & target_mask != 0 {
            attackers.push((from, PieceKind::Rook));
        }
        rooks &= rooks - 1;
    }

    let mut queens = game_state.pieces[attacker_color.index()][PieceKind::Queen.index()];
    while queens != 0 {
        let from = queens.trailing_zeros() as Square;
        let attacks = bishop_attacks(from, game_state.occupancy_all)
            | rook_attacks(from, game_state.occupancy_all);
        if attacks & target_mask != 0 {
            attackers.push((from, PieceKind::Queen));
        }
        queens &= queens - 1;
    }

    let mut kings = game_state.pieces[attacker_color.index()][PieceKind::King.index()];
    while kings != 0 {
        let from = kings.trailing_zeros() as Square;
        if king_attacks(from) & target_mask != 0 {
            attackers.push((from, PieceKind::King));
        }
        kings &= kings - 1;
    }

    attackers
}
