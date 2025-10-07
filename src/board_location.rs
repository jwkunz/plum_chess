use std::fmt;

use crate::chess_errors::ChessErrors;

#[derive(Clone, Copy)]
pub struct BoardLocation {
    pub binary_location: u64,
}

/// Helper to convert file/rank chars to BoardLocation.
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
    pub fn to_long_algebraic(&self) -> String {
        let (file, rank) = self.get_file_rank();
        let fs = (b'a' + file) as char;
        let rs = (b'1' + rank) as char;
        format!("{}{}", fs, rs)
    }
    pub fn from_file_rank(file: u8, rank: u8) -> Result<Self, ChessErrors> {
        if file <= 8 && rank <= 8 {
            Ok(BoardLocation {
                binary_location: (1 as u64) << (8 * file) + rank,
            })
        } else {
            Err(ChessErrors::InvalidFileOrRank((file, rank)))
        }
    }
    pub fn get_file_rank(&self) -> (u8, u8) {
        let bit_spot = self.binary_location.ilog2();
        ((bit_spot / 8) as u8, (bit_spot % 8) as u8)
    }
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
    pub fn generate_moved_location_without_validation(
        &self,
        d_file: i8,
        d_rank: i8,
    ) -> BoardLocation {
        let shift_amount = (8 * d_file) + d_rank;
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
    }
}
