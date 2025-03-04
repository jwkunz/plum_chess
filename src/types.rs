use crate::errors::*;

#[derive(Copy, Clone)]
pub enum Class {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Affiliation {
    Dark,
    Light,
}

pub type BoardLocation = (i8, i8);

/// Moves a board location by a specified file and rank offset.
///
/// # Arguments
///
/// * `x` - The current board location.
/// * `d_file` - The file offset.
/// * `d_rank` - The rank offset.
///
/// # Returns
///
/// * `Result<BoardLocation, Errors>` - Returns the new board location if within bounds, otherwise returns an error.
pub fn move_board_location(
    x: BoardLocation,
    d_file: i8,
    d_rank: i8,
) -> Result<BoardLocation, Errors> {
    let y: BoardLocation = (x.0 + d_file, x.1 + d_rank);
    if (y.0 < 0) | (y.0 > 7) | (y.1 < 0) | (y.1 > 7) {
        Err(Errors::OutOfBounds)
    } else {
        Ok(y)
    }
}

#[derive(Copy, Clone)]
pub struct PieceRecord {
    pub class: Class,
    pub affiliation: Affiliation,
}

#[derive(Default, Clone)]
pub struct PieceRegister {
    buffer: [[Option<PieceRecord>; 8]; 8],
}

impl PieceRegister {
    /// Returns a mutable reference to the piece record at the specified board location.
    ///
    /// # Arguments
    ///
    /// * `x` - The board location.
    ///
    /// # Returns
    ///
    /// * `&mut Option<PieceRecord>` - A mutable reference to the piece record at the specified location.
    pub fn at(&mut self, x: BoardLocation) -> &mut Option<PieceRecord> {
        &mut self.buffer[x.0 as usize][x.1 as usize]
    }

    /// Returns a reference to the piece record at the specified board location.
    ///
    /// # Arguments
    ///
    /// * `x` - The board location.
    ///
    /// # Returns
    ///
    /// * `&Option<PieceRecord>` - A reference to the piece record at the specified location.
    pub fn view(&self, x: BoardLocation) -> &Option<PieceRecord> {
        &self.buffer[x.0 as usize][x.1 as usize]
    }

    /// Adds a piece record to the specified board location.
    ///
    /// # Arguments
    ///
    /// * `x` - The piece record to add.
    /// * `y` - The board location to add the piece record to.
    ///
    /// # Returns
    ///
    /// * `Result<(), Errors>` - Returns `Ok(())` if the piece record was added successfully, otherwise returns an error.
    pub fn add_piece_record(&mut self, x: PieceRecord, y: BoardLocation) -> Result<(), Errors> {
        let _z = self.at(y);
        if _z.is_some() {
            return Err(Errors::BoardLocationOccupied);
        }
        *self.at(y) = Some(x);
        Ok(())
    }

    /// Removes a piece record from the specified board location.
    ///
    /// # Arguments
    ///
    /// * `y` - The board location to remove the piece record from.
    ///
    /// # Returns
    ///
    /// * `Option<PieceRecord>` - Returns the removed piece record if there was one, otherwise returns `None`.
    pub fn remove_piece_record(&mut self, y: BoardLocation) -> Option<PieceRecord> {
        let z = *self.view(y);
        *self.at(y) = None;
        z
    }

    /// Returns an iterator over the piece records in the buffer.
    ///
    /// # Returns
    ///
    /// * `PieceRegisterIter` - An iterator over the piece records in the buffer.
    pub fn iter(&self) -> PieceRegisterIter {
        PieceRegisterIter {
            register: self,
            x: 0,
            y: 0,
        }
    }
}

/// An iterator over the piece records in the buffer.
pub struct PieceRegisterIter<'a> {
    register: &'a PieceRegister,
    x: usize,
    y: usize,
}

impl<'a> Iterator for PieceRegisterIter<'a> {
    type Item = (BoardLocation, &'a PieceRecord);

    fn next(&mut self) -> Option<Self::Item> {
        while self.y < 8 {
            if let Some(piece) = &self.register.buffer[self.x][self.y] {
                let location = (self.x as i8, self.y as i8);
                self.x += 1;
                if self.x == 8 {
                    self.x = 0;
                    self.y += 1;
                }
                return Some((location, piece));
            }
            self.x += 1;
            if self.x == 8 {
                self.x = 0;
                self.y += 1;
            }
        }
        None
    }
}
