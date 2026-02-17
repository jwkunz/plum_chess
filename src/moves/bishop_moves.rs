//! Bishop attack bitboard generation utilities.
//!
//! Provides precomputed and/or occupancy-aware attack maps used by legal move
//! generation and tactical evaluation. These routines are performance-critical
//! building blocks for both perft and search.

pub const BISHOP_RAYS: [u64; 64] = generate_bishop_rays();

#[inline]
pub fn bishop_attacks(square: u8, occupancy: u64) -> u64 {
    let sq = square as i32;
    let mut attacks = 0u64;

    attacks |= trace_ray(sq, 1, 1, occupancy);
    attacks |= trace_ray(sq, -1, 1, occupancy);
    attacks |= trace_ray(sq, 1, -1, occupancy);
    attacks |= trace_ray(sq, -1, -1, occupancy);

    attacks
}

const fn generate_bishop_rays() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        let sq_i = sq as i32;
        let mut rays = 0u64;

        rays |= trace_ray_const(sq_i, 1, 1);
        rays |= trace_ray_const(sq_i, -1, 1);
        rays |= trace_ray_const(sq_i, 1, -1);
        rays |= trace_ray_const(sq_i, -1, -1);

        table[sq] = rays;
        sq += 1;
    }

    table
}

fn trace_ray(square: i32, file_step: i32, rank_step: i32, occupancy: u64) -> u64 {
    let mut file = (square % 8) + file_step;
    let mut rank = (square / 8) + rank_step;
    let mut attacks = 0u64;

    while (0..8).contains(&file) && (0..8).contains(&rank) {
        let target = (rank * 8 + file) as usize;
        let bit = 1u64 << target;
        attacks |= bit;

        if (occupancy & bit) != 0 {
            break;
        }

        file += file_step;
        rank += rank_step;
    }

    attacks
}

const fn trace_ray_const(square: i32, file_step: i32, rank_step: i32) -> u64 {
    let mut file = (square % 8) + file_step;
    let mut rank = (square / 8) + rank_step;
    let mut attacks = 0u64;

    while file >= 0 && file < 8 && rank >= 0 && rank < 8 {
        let target = (rank * 8 + file) as usize;
        attacks |= 1u64 << target;
        file += file_step;
        rank += rank_step;
    }

    attacks
}

#[cfg(test)]
mod tests {
    use super::{bishop_attacks, BISHOP_RAYS};

    #[test]
    fn bishop_rays_from_d4_have_thirteen_squares() {
        let d4 = 27u8;
        assert_eq!(BISHOP_RAYS[d4 as usize].count_ones(), 13);
    }

    #[test]
    fn bishop_blocker_stops_ray() {
        let c1 = 2u8;
        let blocker_on_e3 = 1u64 << 20;
        let attacks = bishop_attacks(c1, blocker_on_e3);

        assert_ne!(attacks & (1u64 << 20), 0);
        assert_eq!(attacks & (1u64 << 29), 0);
    }
}
