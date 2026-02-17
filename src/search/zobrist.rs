//! Zobrist hashing support for fast position identity and repetition tracking.
//!
//! The keys are generated from a fixed seed so hashes are deterministic across
//! runs, which is useful for testing and debugging.

use std::sync::OnceLock;

use crate::game_state::{chess_types::*, game_state::GameState};

#[derive(Debug)]
struct ZobristTables {
    piece_square: [[[u64; 64]; 6]; 2],
    side_to_move: u64,
    castling: [u64; 16],
    en_passant_file: [u64; 8],
}

static TABLES: OnceLock<ZobristTables> = OnceLock::new();

#[inline]
fn tables() -> &'static ZobristTables {
    TABLES.get_or_init(build_tables)
}

fn build_tables() -> ZobristTables {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;

    let mut piece_square = [[[0u64; 64]; 6]; 2];
    for color in &mut piece_square {
        for piece in color {
            for sq in piece {
                *sq = next_random_u64(&mut seed);
            }
        }
    }

    let side_to_move = next_random_u64(&mut seed);

    let mut castling = [0u64; 16];
    for key in &mut castling {
        *key = next_random_u64(&mut seed);
    }

    let mut en_passant_file = [0u64; 8];
    for key in &mut en_passant_file {
        *key = next_random_u64(&mut seed);
    }

    ZobristTables {
        piece_square,
        side_to_move,
        castling,
        en_passant_file,
    }
}

#[inline]
fn next_random_u64(state: &mut u64) -> u64 {
    // splitmix64
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Return the Zobrist key for a `(color, piece, square)` occupancy term.
#[inline]
pub fn piece_square_key(color: Color, piece: PieceKind, square: Square) -> u64 {
    tables().piece_square[color.index()][piece.index()][square as usize]
}

/// Return the Zobrist key contribution for castling rights mask (`0..=15`).
#[inline]
pub fn castling_key(castling_rights: CastlingRights) -> u64 {
    tables().castling[(castling_rights & 0x0F) as usize]
}

/// Return the Zobrist key contribution for a valid en-passant file.
#[inline]
pub fn en_passant_file_key(file: u8) -> u64 {
    tables().en_passant_file[file as usize]
}

/// Return the side-to-move toggle key (xor in when dark to move).
#[inline]
pub fn side_to_move_key() -> u64 {
    tables().side_to_move
}

/// Compute the full position Zobrist key from the complete game state.
pub fn compute_zobrist_key(game_state: &GameState) -> u64 {
    let mut key = 0u64;

    for color in [Color::Light, Color::Dark] {
        for piece in [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ] {
            let mut bb = game_state.pieces[color.index()][piece.index()];
            while bb != 0 {
                let sq = bb.trailing_zeros() as Square;
                key ^= piece_square_key(color, piece, sq);
                bb &= bb - 1;
            }
        }
    }

    if game_state.side_to_move == Color::Dark {
        key ^= side_to_move_key();
    }

    key ^= castling_key(game_state.castling_rights);

    if let Some(ep_square) = game_state.en_passant_square {
        let file = ep_square % 8;
        key ^= en_passant_file_key(file);
    }

    key
}

/// Compute the pawn-only Zobrist key from pawns and kings.
///
/// This is commonly used for pawn structure caches. Kings are included to keep
/// the key compatible with king-safety dependent pawn evaluations.
pub fn compute_pawn_zobrist_key(game_state: &GameState) -> u64 {
    let mut key = 0u64;

    for color in [Color::Light, Color::Dark] {
        for piece in [PieceKind::Pawn, PieceKind::King] {
            let mut bb = game_state.pieces[color.index()][piece.index()];
            while bb != 0 {
                let sq = bb.trailing_zeros() as Square;
                key ^= piece_square_key(color, piece, sq);
                bb &= bb - 1;
            }
        }
    }

    key
}

/// Recompute and store both incremental hash fields on the provided state.
#[inline]
pub fn refresh_game_state_hashes(game_state: &mut GameState) {
    game_state.zobrist_key = compute_zobrist_key(game_state);
    game_state.pawn_zobrist_key = compute_pawn_zobrist_key(game_state);
}

#[cfg(test)]
mod tests {
    use super::{compute_zobrist_key, refresh_game_state_hashes};
    use crate::game_state::game_state::GameState;
    use crate::move_generation::legal_move_apply::apply_move;
    use crate::utils::long_algebraic::long_algebraic_to_move_description;

    #[test]
    fn starting_position_hash_is_deterministic() {
        let a = GameState::new_game();
        let b = GameState::new_game();
        assert_eq!(a.zobrist_key, b.zobrist_key);
        assert_eq!(a.pawn_zobrist_key, b.pawn_zobrist_key);
    }

    #[test]
    fn side_to_move_changes_hash() {
        let w = GameState::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").expect("FEN should parse");
        let b = GameState::from_fen("4k3/8/8/8/8/8/8/4K3 b - - 0 1").expect("FEN should parse");
        assert_ne!(w.zobrist_key, b.zobrist_key);
    }

    #[test]
    fn castling_rights_change_hash() {
        let with_rights =
            GameState::from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").expect("FEN should parse");
        let without_rights =
            GameState::from_fen("4k3/8/8/8/8/8/8/R3K2R w - - 0 1").expect("FEN should parse");
        assert_ne!(with_rights.zobrist_key, without_rights.zobrist_key);
    }

    #[test]
    fn en_passant_file_changes_hash() {
        let no_ep =
            GameState::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").expect("FEN should parse");
        let ep = GameState::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - e3 0 1").expect("FEN should parse");
        assert_ne!(no_ep.zobrist_key, ep.zobrist_key);
    }

    #[test]
    fn recompute_matches_after_apply_move() {
        let game = GameState::new_game();
        let mv = long_algebraic_to_move_description("e2e4", &game).expect("move should parse");
        let mut next = apply_move(&game, mv).expect("move should apply");
        let recomputed = compute_zobrist_key(&next);
        assert_eq!(next.zobrist_key, recomputed);

        next.castling_rights ^= 1;
        refresh_game_state_hashes(&mut next);
        assert_eq!(next.zobrist_key, compute_zobrist_key(&next));
    }
}
