use std::collections::HashMap;

use crate::{board_location::{BinaryLocation, BoardLocation}, board_mask::BoardMask, chess_errors::ChessErrors, piece_record::PieceRecord, piece_team::PieceTeam};

/// A registry that tracks all pieces currently on the chessboard.
///
/// PieceRegister maintains two separate hash maps for light and dark pieces
/// and optional cached keys for each king's binary board location. The maps
/// are keyed by BinaryLocation (u64) and store PieceRecord values describing
/// each piece's type, team, and BoardLocation.
///
/// The register is responsible for:
/// - Providing masks of occupied squares for either team or both teams.
/// - Looking up and mutably editing pieces by BoardLocation.
/// - Moving pieces (including capturing and king key updates).
/// - Adding and removing pieces without applying chess rules (helper functions
///   used by higher-level game logic which enforces rules).
///
/// Note: public methods return ChessErrors for failure cases so callers can
/// react appropriately (for example, when querying empty squares or trying to
/// remove a non-existent piece).
#[derive(Clone, Debug)]
pub struct PieceRegister {
	/// BinaryLocation of the light king, if present.
	///
	/// Stored as an Option to indicate the king may not currently be in the
	/// register (for example during construction or special test setups).
	light_king_key : Option<BinaryLocation>,

	/// BinaryLocation of the dark king, if present.
	dark_king_key : Option<BinaryLocation>,

	/// Mapping of light-side pieces keyed by their BinaryLocation.
	///
	/// Public for read access by other modules/tests. Values are PieceRecord
	/// instances describing class, team and exact BoardLocation.
	pub light_pieces : HashMap<u64,PieceRecord>,

	/// Mapping of dark-side pieces keyed by their BinaryLocation.
	pub dark_pieces : HashMap<u64,PieceRecord>,
}

impl PieceRegister {
	/// Create a new, empty PieceRegister.
	///
	/// The returned register contains empty maps for both teams and no king
	/// keys. The internal HashMaps are pre-allocated with a capacity suitable
	/// for a standard chess set to avoid immediate reallocation.
	pub fn new() -> Self{
		let light_pieces = HashMap::<u64,PieceRecord>::with_capacity(18);
		let dark_pieces = HashMap::<u64,PieceRecord>::with_capacity(18);
		let light_king_key=None;
		let dark_king_key=None;
		PieceRegister { 
			light_king_key, 
			dark_king_key, 
			light_pieces, 
			dark_pieces}
	}

	/// Generate a BoardMask containing all squares occupied by light pieces.
	///
	/// Returns a bitmask (BoardMask) where each bit corresponds to a square
	/// occupied by a light piece. If there are no light pieces this returns 0.
	pub fn generate_mask_all_light(&self)->BoardMask{
		let mut result = 0;
		for (_,value) in &self.light_pieces{
			result |= value.location.binary_location;
		}
		result
	}

	/// Generate a BoardMask containing all squares occupied by dark pieces.
	///
	/// Similar to generate_mask_all_light but for the dark side.
	pub fn generate_mask_all_dark(&self)->BoardMask{
		let mut result = 0;
		for (_,value) in &self.dark_pieces{
			result |= value.location.binary_location;
		}
		result
	}  

	/// Generate a BoardMask containing all occupied squares (both teams).
	///
	/// Combines the results of generate_mask_all_dark and generate_mask_all_light.
	pub fn generate_mask_all_pieces(&self)->BoardMask{
		self.generate_mask_all_dark() | self.generate_mask_all_light()
	}  

	/// Return a mask for the light king's square.
	///
	/// If the light king has not been registered this returns
	/// ChessErrors::PieceRegisterDoesNotContainAKing.
	pub fn generate_mask_light_king(&self)->Result<BoardMask,ChessErrors>{
		Ok(self.view_king(PieceTeam::Light)?.location.binary_location)
	}

	/// Return a mask for the dark king's square.
	///
	/// If the dark king has not been registered this returns
	/// ChessErrors::PieceRegisterDoesNotContainAKing.
	pub fn generate_mask_dark_king(&self)->Result<BoardMask,ChessErrors>{
		Ok(self.view_king(PieceTeam::Dark)?.location.binary_location)
	}    

	/// View a piece at a given BoardLocation by reference.
	///
	/// Returns Ok(&PieceRecord) when a piece exists at the requested location.
	/// Returns Err(ChessErrors::TryToViewOrEditEmptySquare) when the square is
	/// empty (neither light nor dark map contains the location).
	pub fn view_piece_at_location(&self, x : BoardLocation) -> Result<&PieceRecord, ChessErrors>{
		if let Some(piece) = self.light_pieces.get(&x.binary_location){
			return Ok(piece);
		}else if let Some(piece) = self.dark_pieces.get(&x.binary_location){
			return Ok(piece);
		}
		Err(ChessErrors::TryToViewOrEditEmptySquare(x))
	}

	/// View the king PieceRecord for the given team.
	///
	/// Returns Ok(&PieceRecord) for the requested king. Returns
	/// ChessErrors::PieceRegisterDoesNotContainAKing if the king key is None
	/// or if the king is not present in the corresponding map.
	pub fn view_king(&self, x : PieceTeam) -> Result<&PieceRecord, ChessErrors>{
		match x{
			PieceTeam::Light =>{
			Ok(&self.light_pieces[&self.light_king_key.ok_or_else(|| ChessErrors::PieceRegisterDoesNotContainAKing)?])
			}
			PieceTeam::Dark =>{
			Ok(&self.dark_pieces[&self.dark_king_key.ok_or_else(|| ChessErrors::PieceRegisterDoesNotContainAKing)?])
			}
		}
	}   

	/// Mutably borrow a PieceRecord at the given BoardLocation.
	///
	/// Useful for in-place updates (for example, updating moved flags or
	/// doubling a pawn). Returns Err(ChessErrors::TryToViewOrEditEmptySquare)
	/// if there is no piece at the requested location.
	pub fn edit_piece_at_location(&mut self, x : BoardLocation) -> Result<&mut PieceRecord, ChessErrors>{
		if let Some(piece) = self.light_pieces.get_mut(&x.binary_location){
			return Ok(piece);
		}else if let Some(piece) = self.dark_pieces.get_mut(&x.binary_location){
			return Ok(piece);
		}
		Err(ChessErrors::TryToViewOrEditEmptySquare(x))
	}    

	/// Move a piece from `start` to `destination`, overwriting any piece at the destination.
	///
	/// This function:
	/// - Removes the piece at `start` (returns an error if start is empty).
	/// - Updates the removed piece's location to `destination`.
	/// - Inserts the piece into the appropriate team map at `destination`.
	/// - If the moved piece is a king, updates the stored king key for that team.
	/// - If an opponent piece existed at `destination`, it is removed and
	///   returned as Some(PieceRecord). If no capture occurred, returns Ok(None).
	///
	/// Returns ChessErrors variants as returned by remove_piece_at_location when
	/// the start square is empty.
	pub fn move_piece_to_location_with_overwrite(&mut self, start : BoardLocation, destination : BoardLocation) -> Result<Option<PieceRecord>, ChessErrors>{
		let mut start_piece = self.remove_piece_at_location(start)?;
		start_piece.location = destination;
		let captured = match start_piece.team {
			crate::piece_team::PieceTeam::Light => {
				if matches!(start_piece.class, crate::piece_class::PieceClass::King){
					self.light_king_key = Some(destination.binary_location);
				}
				self.light_pieces.insert(destination.binary_location, start_piece);
				self.dark_pieces.remove(&destination.binary_location)
			},
			crate::piece_team::PieceTeam::Dark => {
				if matches!(start_piece.class, crate::piece_class::PieceClass::King){
					self.dark_king_key = Some(destination.binary_location);
				}     
				self.dark_pieces.insert(destination.binary_location, start_piece);
				self.light_pieces.remove(&destination.binary_location)
			},
		};
		Ok(captured)
	}     

	/// Remove and return the PieceRecord at the given location.
	///
	/// If a king is removed, the corresponding king key is cleared. Returns
	/// Err(ChessErrors::CannotRemoveFromEmptyLocation) if the requested
	/// location does not contain a piece.
	pub fn remove_piece_at_location(&mut self, x : BoardLocation) -> Result<PieceRecord,ChessErrors>{
		if self.light_king_key == Some(x.binary_location){
			self.light_king_key = None;
		}
		if self.dark_king_key == Some(x.binary_location){
			self.dark_king_key = None;
		}
		if let Some(piece) = self.light_pieces.remove(&x.binary_location){
			return Ok(piece);
		}else if let Some(piece) = self.dark_pieces.remove(&x.binary_location){
			return Ok(piece);
		}
		Err(ChessErrors::CannotRemoveFromEmptyLocation(x))
	}    

	/// Add a PieceRecord without applying chess rules or validation.
	///
	/// This helper inserts the provided PieceRecord into the appropriate team
	/// map and updates the king key if the added piece is a king. It does not
	/// check for collisions or legal starting positions; callers must ensure
	/// the inserted piece makes sense for the current game state.
	pub fn add_piece_record_no_rule_checking(&mut self, x : PieceRecord){
		match x.team {
			crate::piece_team::PieceTeam::Light => {
				if matches!(x.class , crate::piece_class::PieceClass::King){
					self.light_king_key = Some(x.location.binary_location);
				}
				self.light_pieces.insert(x.location.binary_location,x);
			},
			crate::piece_team::PieceTeam::Dark =>{
				if matches!(x.class , crate::piece_class::PieceClass::King){
					self.dark_king_key = Some(x.location.binary_location);
				}
				self.dark_pieces.insert(x.location.binary_location,x);
			}
		}
	}
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn add_remove_pieces() -> Result<(),ChessErrors>{
        let mut dut = PieceRegister::new();
        dut.add_piece_record_no_rule_checking(PieceRecord { class: crate::piece_class::PieceClass::Pawn, location: BoardLocation::from_file_rank(0, 1).unwrap(), team: crate::piece_team::PieceTeam::Light });
        dut.add_piece_record_no_rule_checking(PieceRecord { class: crate::piece_class::PieceClass::Pawn, location: BoardLocation::from_file_rank(0, 2).unwrap(), team: crate::piece_team::PieceTeam::Light });
        let _ = dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 1).unwrap())?;
        let _ = dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 2).unwrap())?;
        if dut.remove_piece_at_location(BoardLocation::from_file_rank(0, 1).unwrap()).is_err(){
            return Ok(())
        }
        Err(ChessErrors::FailedTest)
    }
}



