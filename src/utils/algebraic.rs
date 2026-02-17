//! Square and bitboard conversions for long algebraic coordinates.
//!
//! Converts between human-readable coordinates (e.g., `e4`) and internal
//! square/bitboard representations reused by FEN/PGN/UCI components.

use crate::game_state::chess_types::Square;

/// Convert long algebraic notation (for example: "e4") to a square index.
#[inline]
pub fn algebraic_to_square(square: &str) -> Result<Square, String> {
    let bytes = square.as_bytes();
    if bytes.len() != 2 {
        return Err(format!("Invalid algebraic square: {square}"));
    }

    let file = bytes[0];
    let rank = bytes[1];

    if !(b'a'..=b'h').contains(&file) {
        return Err(format!("Invalid algebraic file: {}", file as char));
    }
    if !(b'1'..=b'8').contains(&rank) {
        return Err(format!("Invalid algebraic rank: {}", rank as char));
    }

    let file_index = file - b'a';
    let rank_index = rank - b'1';
    Ok(rank_index * 8 + file_index)
}

/// Convert long algebraic notation (for example: "e4") to a one-hot bitboard.
#[inline]
pub fn algebraic_to_bitboard(square: &str) -> Result<u64, String> {
    let index = algebraic_to_square(square)?;
    Ok(1u64 << index)
}

/// Convert a square index (`0..=63`) to long algebraic notation (for example: "e4").
#[inline]
pub fn square_to_algebraic(square: Square) -> Result<String, String> {
    if square > 63 {
        return Err(format!("Square index out of bounds: {square}"));
    }

    let file = square % 8;
    let rank = square / 8;
    let file_char = char::from(b'a' + file);
    let rank_char = char::from(b'1' + rank);

    Ok(format!("{file_char}{rank_char}"))
}

/// Convert a one-hot bitboard to long algebraic notation (for example: "e4").
#[inline]
pub fn bitboard_to_algebraic(bitboard: u64) -> Result<String, String> {
    if bitboard == 0 {
        return Err("Bitboard must contain exactly one set bit, got empty bitboard".to_owned());
    }
    if bitboard.count_ones() != 1 {
        return Err(format!(
            "Bitboard must contain exactly one set bit, got {}",
            bitboard.count_ones()
        ));
    }

    let square = bitboard.trailing_zeros() as Square;
    square_to_algebraic(square)
}

#[cfg(test)]
mod tests {
    use super::{
        algebraic_to_bitboard, algebraic_to_square, bitboard_to_algebraic, square_to_algebraic,
    };

    #[test]
    fn round_trip_square_conversions() {
        assert_eq!(algebraic_to_square("a1").expect("a1 should parse"), 0);
        assert_eq!(algebraic_to_square("h8").expect("h8 should parse"), 63);
        assert_eq!(square_to_algebraic(0).expect("0 should convert"), "a1");
        assert_eq!(square_to_algebraic(63).expect("63 should convert"), "h8");
    }

    #[test]
    fn round_trip_bitboard_conversion() {
        let e4 = algebraic_to_bitboard("e4").expect("e4 should parse");
        assert_eq!(e4, 1u64 << 28);
        assert_eq!(
            bitboard_to_algebraic(e4).expect("one-hot bitboard should convert"),
            "e4"
        );
    }
}
