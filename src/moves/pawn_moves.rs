use crate::game_state::chess_types::Color;

pub const LIGHT_PAWN_ATTACKS: [u64; 64] = generate_light_pawn_attacks();
pub const DARK_PAWN_ATTACKS: [u64; 64] = generate_dark_pawn_attacks();

#[inline]
pub const fn pawn_attacks(color: Color, square: u8) -> u64 {
    match color {
        Color::Light => LIGHT_PAWN_ATTACKS[square as usize],
        Color::Dark => DARK_PAWN_ATTACKS[square as usize],
    }
}

const fn generate_light_pawn_attacks() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        let file = sq % 8;
        let rank = sq / 8;
        let mut attacks = 0u64;

        if rank < 7 {
            if file > 0 {
                attacks |= 1u64 << (sq + 7);
            }
            if file < 7 {
                attacks |= 1u64 << (sq + 9);
            }
        }

        table[sq] = attacks;
        sq += 1;
    }

    table
}

const fn generate_dark_pawn_attacks() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;

    while sq < 64 {
        let file = sq % 8;
        let rank = sq / 8;
        let mut attacks = 0u64;

        if rank > 0 {
            if file > 0 {
                attacks |= 1u64 << (sq - 9);
            }
            if file < 7 {
                attacks |= 1u64 << (sq - 7);
            }
        }

        table[sq] = attacks;
        sq += 1;
    }

    table
}

#[cfg(test)]
mod tests {
    use super::{pawn_attacks, DARK_PAWN_ATTACKS, LIGHT_PAWN_ATTACKS};
    use crate::game_state::chess_types::Color;

    #[test]
    fn light_pawn_attacks_from_e2() {
        let e2 = 12u8;
        let expected = (1u64 << 19) | (1u64 << 21);
        assert_eq!(LIGHT_PAWN_ATTACKS[e2 as usize], expected);
        assert_eq!(pawn_attacks(Color::Light, e2), expected);
    }

    #[test]
    fn dark_pawn_attacks_from_e7() {
        let e7 = 52u8;
        let expected = (1u64 << 43) | (1u64 << 45);
        assert_eq!(DARK_PAWN_ATTACKS[e7 as usize], expected);
        assert_eq!(pawn_attacks(Color::Dark, e7), expected);
    }
}
