//! Chess board location representation and utilities.
//! 
//! This module provides the BoardLocation type which represents a square on a chess board
//! using an efficient binary representation. The implementation uses a u64 where each bit
//! represents one square on the 8x8 chess board, with the least significant bit being a1
//! and the most significant bit being h8.
//!
//! Key features:
//! - Efficient binary representation using a single u64
//! - Conversion between algebraic notation (e.g. "e4") and internal representation
//! - Safe and unsafe move generation utilities
//! - File/rank coordinate system with zero-based indexing (0-7 for both)
//!
//! The coordinate system uses:
//! - Files: 0-7 representing a-h from left to right
//! - Ranks: 0-7 representing 1-8 from bottom to top
//! 
//! Example:
//! ```
//! use plum_chess::BoardLocation;
//! 
//! // Create a location from algebraic notation
//! let e4 = BoardLocation::from_long_algebraic("e4").unwrap();
//! 
//! // Create a location from file/rank coordinates
//! let same_square = BoardLocation::from_file_rank(4, 3).unwrap();
//! 
//! assert_eq!(e4.binary_location, same_square.binary_location);
//! ```

use std::fmt;
use crate::chess_errors::ChessErrors;

/// Type alias for the underlying binary board location representation.
/// Uses a u64 where each bit represents one square on the 8x8 board.
pub type BinaryLocation = u64;

/// Represents a square on a chess board using a binary representation.
///
/// The binary representation uses a u64 where each bit corresponds to one square.
/// The least significant bit represents a1, and bits increment first by rank
/// then by file, with the most significant bit representing h8.
///
/// This structure provides methods for:
/// - Creating locations from algebraic notation or file/rank coordinates
/// - Converting locations to algebraic notation
/// - Generating new locations by applying moves
/// - Accessing the underlying file/rank coordinates
#[derive(Clone, Copy)]
pub struct BoardLocation {
    /// The binary representation of the board location.
    /// Only one bit should be set at a time, representing the square's position.
    pub binary_location: BinaryLocation,
}

/// Helper function to convert algebraic notation characters to zero-based indices.
///
/// Takes a file character (a-h) and rank character (1-8) and converts them to
/// zero-based indices suitable for internal use.
///
/// # Arguments
/// * `file` - A character representing the file ('a' through 'h')
/// * `rank` - A character representing the rank ('1' through '8')
///
/// # Returns
/// * `Ok((file_idx, rank_idx))` - Tuple of zero-based indices (0-7)
/// * `Err(ChessErrors::InvalidAlgebraicChar)` - If either character is invalid
fn parse_square(file: char, rank: char) -> Result<(u8, u8), ChessErrors> {
    let file_idx = match file {
        'a'..='h' => (file as u8 - b'a') as u8,
        _ => return Err(ChessErrors::InvalidAlgebraicChar(file)),
    };
    let rank_idx = match rank {
        '1'..='8' => (rank as u8 - b'1') as u8,
        _ => return Err(ChessErrors::InvalidAlgebraicChar(rank)),
    };
    Ok((file_idx, rank_idx))
}

impl BoardLocation {
    /// Creates a new BoardLocation from algebraic notation (e.g. "e4").
    ///
    /// # Arguments
    /// * `x` - A string slice containing exactly 2 characters representing
    ///         the algebraic notation (e.g. "e4", "a1", etc.)
    ///
    /// # Returns
    /// * `Ok(BoardLocation)` - Successfully parsed location
    /// * `Err(ChessErrors)` - If the string is invalid or coordinates are out of bounds
    ///
    /// # Examples
    /// ```
    /// let e4 = BoardLocation::from_long_algebraic("e4").unwrap();
    /// assert_eq!(e4.to_long_algebraic(), "e4");
    /// ```
    pub fn from_long_algebraic(x: &str) -> Result<Self, ChessErrors> {
        // Must be 2 chars (e.g., e2e4)
        let x = x.trim();
        if x.len() != 2 {
            return Err(ChessErrors::InvalidAlgebraicString(x.into()));
        }
        let bytes = x.as_bytes();
        let (file, rank) = parse_square(bytes[0] as char, bytes[1] as char)?;
        BoardLocation::from_file_rank(file, rank)
    }

    /// Converts the location to algebraic notation (e.g. "e4").
    ///
    /// # Returns
    /// * String containing the two-character algebraic notation
    pub fn to_long_algebraic(&self) -> String {
        let (file, rank) = self.get_file_rank();
        let fs = (b'a' + file) as char;
        let rs = (b'1' + rank) as char;
        format!("{}{}", fs, rs)
    }

    /// Creates a new BoardLocation from file and rank indices.
    ///
    /// # Arguments
    /// * `file` - Zero-based file index (0-7 representing a-h)
    /// * `rank` - Zero-based rank index (0-7 representing 1-8)
    ///
    /// # Returns
    /// * `Ok(BoardLocation)` - Successfully created location
    /// * `Err(ChessErrors::InvalidFileOrRank)` - If indices are out of bounds
    pub fn from_file_rank(file: u8, rank: u8) -> Result<Self, ChessErrors> {
        if file <= 8 && rank <= 8 {
            Ok(BoardLocation {
                binary_location: (1 as u64) << (8 * file) + rank,
            })
        } else {
            Err(ChessErrors::InvalidFileOrRank((file, rank)))
        }
    }

    /// Gets the file and rank indices for this location.
    ///
    /// # Returns
    /// * `(file, rank)` - Tuple of zero-based indices (0-7)
    pub fn get_file_rank(&self) -> (u8, u8) {
        let bit_spot = self.binary_location.ilog2();
        ((bit_spot / 8) as u8, (bit_spot % 8) as u8)
    }

    /// Generates a new location by applying a move, with bounds checking.
    ///
    /// # Arguments
    /// * `d_file` - Change in file (-7 to 7)
    /// * `d_rank` - Change in rank (-7 to 7)
    ///
    /// # Returns
    /// * `Ok(BoardLocation)` - Successfully generated new location
    /// * `Err(ChessErrors::TriedToMoveOutOfBounds)` - If move would leave the board
    pub fn generate_moved_location_checked(
        &self,
        d_file: i8,
        d_rank: i8,
    ) -> Result<BoardLocation, ChessErrors> {
        let (f, r) = self.get_file_rank();
        let f_next = f as i8 + d_file;
        if f_next < 0 || f_next > 7 {
            return Err(ChessErrors::TriedToMoveOutOfBounds((
                self.clone(),
                d_file,
                d_rank,
            )));
        }
        let r_next = r as i8 + d_rank;
        if r_next < 0 || r_next > 7 {
            return Err(ChessErrors::TriedToMoveOutOfBounds((
                self.clone(),
                d_file,
                d_rank,
            )));
        }
        BoardLocation::from_file_rank(f_next as u8, r_next as u8)
    }

    /// Generates a new location by applying a move, without bounds checking.
    ///
    /// This is a faster version of move generation that doesn't perform bounds
    /// checking. It should only be used when the caller has already verified
    /// the move is valid.
    ///
    /// # Arguments
    /// * `d_file` - Change in file (-7 to 7)
    /// * `d_rank` - Change in rank (-7 to 7)
    ///
    /// # Returns
    /// * `BoardLocation` - The new location (may be invalid if move was illegal)
    ///
    /// # Safety
    /// This function does not check if the resulting location is valid. The caller
    /// must ensure the move stays within board bounds (0-7 for both file and rank).
    pub fn generate_moved_location_without_validation(
        &self,
        d_file: i8,
        d_rank: i8,
    ) -> BoardLocation {
        let shift_amount = (d_file << 3) + d_rank;
        let new_location = if shift_amount >= 0 {
            self.binary_location.wrapping_shl(shift_amount as u32)
        } else {
            self.binary_location.wrapping_shr(shift_amount.abs() as u32)
        };
        BoardLocation {
            binary_location: new_location,
        }
    }
}

// Provide a custom Debug impl that prints the square as algebraic coords like "e2".
impl fmt::Debug for BoardLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BoardLocation({}) is {}",
            self.binary_location,
            self.to_long_algebraic()
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_file_rank_into_board_location() {
        let mut dut = BoardLocation::from_file_rank(0, 0).unwrap();
        assert_eq!(dut.binary_location, 1);
        let (f, r) = dut.get_file_rank();
        assert_eq!(f, 0);
        assert_eq!(r, 0);

        dut = BoardLocation::from_file_rank(7, 7).unwrap();
        assert_eq!(dut.binary_location, 0x8000_0000_0000_0000 as u64);
        let (f, r) = dut.get_file_rank();
        assert_eq!(f, 7);
        assert_eq!(r, 7);

        dut = BoardLocation::from_file_rank(7, 0).unwrap();
        assert_eq!(dut.binary_location, 0x0100_0000_0000_0000 as u64);
        let (f, r) = dut.get_file_rank();
        assert_eq!(f, 7);
        assert_eq!(r, 0);

        dut = BoardLocation::from_file_rank(0, 7).unwrap();
        assert_eq!(dut.binary_location, 0x0000_0000_0000_0080 as u64);
        let (f, r) = dut.get_file_rank();
        assert_eq!(f, 0);
        assert_eq!(r, 7);

        dut = BoardLocation::from_file_rank(1, 0).unwrap();
        assert_eq!(dut.binary_location, 0x0000_0000_0000_0100 as u64);
        let (f, r) = dut.get_file_rank();
        assert_eq!(f, 1);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_move_d_file_and_d_rank() {
        let dut = BoardLocation::from_file_rank(0, 0).unwrap();
        let next_dut = dut.generate_moved_location_without_validation(1, 1);
        let (f, r) = next_dut.get_file_rank();
        assert_eq!(f, 1);
        assert_eq!(r, 1);

        let dut = BoardLocation::from_file_rank(5, 5).unwrap();
        let next_dut = dut.generate_moved_location_without_validation(-1, -4);
        let (f, r) = next_dut.get_file_rank();
        assert_eq!(f, 4);
        assert_eq!(r, 1);

        let dut = BoardLocation::from_file_rank(0, 0).unwrap();
        assert!(dut.generate_moved_location_checked(-1, 1).is_err());
    }
}
