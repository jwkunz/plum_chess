pub const KNIGHT_ATTACKS: [u64; 64] = generate_knight_attacks();

#[inline]
pub const fn knight_attacks(square: u8) -> u64 {
    KNIGHT_ATTACKS[square as usize]
}

const fn generate_knight_attacks() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut attacks = 0u64;

        attacks |= set_if_valid(file + 1, rank + 2);
        attacks |= set_if_valid(file + 2, rank + 1);
        attacks |= set_if_valid(file + 2, rank - 1);
        attacks |= set_if_valid(file + 1, rank - 2);
        attacks |= set_if_valid(file - 1, rank - 2);
        attacks |= set_if_valid(file - 2, rank - 1);
        attacks |= set_if_valid(file - 2, rank + 1);
        attacks |= set_if_valid(file - 1, rank + 2);

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
    use super::{knight_attacks, KNIGHT_ATTACKS};

    #[test]
    fn knight_attacks_from_d4_has_eight_targets() {
        let d4 = 27u8;
        assert_eq!(KNIGHT_ATTACKS[d4 as usize].count_ones(), 8);
        assert_eq!(knight_attacks(d4).count_ones(), 8);
    }
}
