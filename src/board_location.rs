use crate::errors::Errors;

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
    x: &BoardLocation,
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