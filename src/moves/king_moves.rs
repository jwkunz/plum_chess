//! King attack bitboard generation utilities.
//!
//! Provides precomputed and/or occupancy-aware attack maps used by legal move
//! generation and tactical evaluation. These routines are performance-critical
//! building blocks for both perft and search.

pub const KING_ATTACKS: [u64; 64] = generate_king_attacks();

#[inline]
pub const fn king_attacks(square: u8) -> u64 {
    KING_ATTACKS[square as usize]
}

const fn generate_king_attacks() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut attacks = 0u64;

        attacks |= set_if_valid(file - 1, rank - 1);
        attacks |= set_if_valid(file, rank - 1);
        attacks |= set_if_valid(file + 1, rank - 1);
        attacks |= set_if_valid(file - 1, rank);
        attacks |= set_if_valid(file + 1, rank);
        attacks |= set_if_valid(file - 1, rank + 1);
        attacks |= set_if_valid(file, rank + 1);
        attacks |= set_if_valid(file + 1, rank + 1);

        table[sq] = attacks;
        sq += 1;
    }

    table
}

const fn set_if_valid(file: i32, rank: i32) -> u64 {
    if file < 0 || file > 7 || rank < 0 || rank > 7 {
        return 0;
    }

    let square = (rank as usize) * 8 + (file as usize);
    1u64 << square
}

#[cfg(test)]
mod tests {
    use super::{king_attacks, KING_ATTACKS};

    #[test]
    fn king_attacks_from_a1_has_three_targets() {
        let a1 = 0u8;
        assert_eq!(KING_ATTACKS[a1 as usize].count_ones(), 3);
        assert_eq!(king_attacks(a1).count_ones(), 3);
    }
}
