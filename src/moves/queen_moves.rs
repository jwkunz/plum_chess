use crate::moves::bishop_moves::{bishop_attacks, BISHOP_RAYS};
use crate::moves::rook_moves::{rook_attacks, ROOK_RAYS};

pub const QUEEN_RAYS: [u64; 64] = generate_queen_rays();

#[inline]
pub fn queen_attacks(square: u8, occupancy: u64) -> u64 {
    bishop_attacks(square, occupancy) | rook_attacks(square, occupancy)
}

const fn generate_queen_rays() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        table[sq] = BISHOP_RAYS[sq] | ROOK_RAYS[sq];
        sq += 1;
    }

    table
}

#[cfg(test)]
mod tests {
    use super::{queen_attacks, QUEEN_RAYS};

    #[test]
    fn queen_rays_from_d4_have_twenty_seven_squares() {
        let d4 = 27u8;
        assert_eq!(QUEEN_RAYS[d4 as usize].count_ones(), 27);
    }

    #[test]
    fn queen_attacks_match_union() {
        let d4 = 27u8;
        let blockers = (1u64 << 43) | (1u64 << 30);
        let attacks = queen_attacks(d4, blockers);

        assert_ne!(attacks & (1u64 << 43), 0);
        assert_ne!(attacks & (1u64 << 30), 0);
        assert_eq!(attacks & (1u64 << 51), 0);
        assert_eq!(attacks & (1u64 << 31), 0);
    }
}
