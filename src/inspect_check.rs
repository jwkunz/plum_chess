//! Check and pin inspection utilities.
//!
//! This module implements the logic for determining whether the side to move is
//! currently in check, and—when requested—classifying the nature of that check.
//! The primary exported function is `inspect_check`, which returns an optional
//! `TypesOfCheck` describing the checking condition (single check, double
//! check, discovery, checkmate, or an unclassified check).  Internally the
//! module:
//! - locates the friendly king for the side to move,
//! - computes a "threat mask" covering squares from which a king could be
//!   attacked (including king/knight/slider directions),
//! - filters the enemy piece set to find those actually attacking or pinning,
//! - classifies attackers into checks vs. pins,
//! - if a last-moved piece is provided, attempts to classify the check further
//!   (single vs. discovery) and determines whether legal responses exist (to
//!   detect checkmate).
//!
//! Design notes and invariants:
//! - The implementation never mutates the provided `GameState`; all simulations
//!   are performed by creating hypothetical next states via
//!   `apply_move_to_game_filtering_no_friendly_check`.
//! - For check classification `inspect_check` optionally accepts the last
//!   moved `PieceRecord`. When supplied, the function will attempt a full
//!   classification (single/discovery/double/checkmate). When `None` is given
//!   the function will only report that a check exists and will produce an
//!   `UnclassifiedCheck` value (useful for fast detection without full
//!   classification).
//! - The function uses level-3 and level-4 move generation helpers to inspect
//!   potential attacker moves and candidate defender moves respectively; any
//!   errors from those helpers are propagated as `ChessErrors`.
//!
//! Complexity:
//! - Building the threat mask is constant work relative to the king (bounded by
//!   fixed-direction generation loops).
//! - Generating defender moves to test escape/legal replies requires calling
//!   the level-4 generator for each friendly piece; this is the most expensive
//!   component and is necessary for correct checkmate determination.
//!
//! Error handling:
//! - All internal errors from piece register access or move-generation are
//!   returned as `Err(ChessErrors)` to the caller.
//!
//! Examples:
//! - To simply test if the current side to move is in check (without
//!   classification):
//!   ```ignore
//!   let maybe_check = inspect_check(&game_state, None)?;
//!   if maybe_check.is_some() { /* in-check */ }
//!   ```
//! - To fully classify a check after applying a move, pass the last moved
//!   piece record:
//!   ```ignore
//!   let classification = inspect_check(&game_state, Some(last_moved_piece))?;
//!   ```
use crate::{apply_move_to_game::apply_move_to_game_filtering_no_friendly_check, board_mask::BoardMask, chess_errors::ChessErrors, collision_masks::{CollisionMasks}, game_state::GameState, generate_movements::{generate_bishop_movement, generate_king_movement, generate_knight_movement, generate_rook_movement}, generate_moves_level_3::GenerateLevel3Result, generate_moves_level_4::generate_moves_level_4, piece_record::PieceRecord, piece_register::{PieceRegister}, types_of_check::TypesOfCheck};

/// Gather enemy pieces that could be giving check or creating pins against the king.
///
/// This helper builds a "threat mask" consisting of:
/// - the king's own square,
/// - all squares a king could move to (adjacent squares),
/// - all squares a knight could occupy to attack the king,
/// - all squares along bishop- and rook-like rays that could contain sliding attackers.
///
/// It then filters the enemy piece list and returns all enemy pieces whose
/// bitmask overlaps the threat mask. The returned list contains `PieceRecord`
/// entries (copied) for further inspection.
///
/// Parameters:
/// - `piece_register`: snapshot of piece placements to search enemy lists from.
/// - `king`: the friendly king's PieceRecord (location/team used to select enemies).
///
/// Returns:
/// - Ok(Vec<PieceRecord>) containing all enemy pieces that lie on the threat
///   mask (these are candidates for checks or pins).
/// - Err(ChessErrors) if underlying generation or register access fails.
///
/// Notes:
/// - This function does not by itself determine whether a candidate actually
///   attacks the king; it simply identifies pieces on directions/locations from
///   which an attack is geometrically possible and therefore worthy of further
///   move-generation checks.
fn find_threatening_pieces(piece_register : &PieceRegister, king : &PieceRecord)-> Result<Vec<PieceRecord>,ChessErrors>{
    let mut threatening_pieces : Vec<PieceRecord> = vec![];
    let mut threat_mask : BoardMask = king.location.binary_location;
    for i in 0..8{
        if let Ok(position) = generate_king_movement(king.location,i){
            threat_mask |= position.binary_location;
        }
        if let Ok(position) = generate_knight_movement(king.location,i){
            threat_mask |= position.binary_location;
        }
    }
    for j in 0..4{
        for i in 0..8{
            if let Ok(position) = generate_bishop_movement(king.location,j,i){
                threat_mask |= position.binary_location;
            }
            if let Ok(position) = generate_rook_movement(king.location,j,i){
                threat_mask |= position.binary_location;
            }
        }
    }  
    let enemy_pieces = match king.team {
        crate::piece_team::PieceTeam::Light => &piece_register.dark_pieces,
        crate::piece_team::PieceTeam::Dark => &piece_register.light_pieces
    };
    for i in enemy_pieces{
        if i.0 & threat_mask > 0{
            threatening_pieces.push(*i.1);
        }
    }
    Ok(threatening_pieces)
}

/// Partition candidate attackers into checking pieces and pinning pieces.
///
/// For each candidate piece in `threatening_pieces` this function generates the
/// piece's level-3 moves (captures and pseudo-legal attacks) and checks whether
/// any of those moves capture the friendly king square. If so the piece is a
/// checking piece; otherwise it is treated as a pinning piece (i.e. it attacks
/// squares relevant to pins but does not directly capture the king).
///
/// Parameters:
/// - `collision_masks`: occupancy masks used by the movement generator.
/// - `king`: the friendly king record (used to compute the king's square mask).
/// - `threatening_pieces`: candidate enemy pieces identified by
///   `find_threatening_pieces`.
///
/// Returns:
/// - Ok((checking_pieces, pinning_pieces)) where each element is a Vec of
///   PieceRecord items. Either vector may be empty.
/// - Err(ChessErrors) if move generation fails for a candidate piece.
///
/// Notes:
/// - The function relies on `GenerateLevel3Result::from` to produce capture
///   targets for each attacker. The precise semantics of level-3 generation
///   (pseudo-legal captures) are expected by callers.
fn sort_threats_to_pins_or_checks(collision_masks : &CollisionMasks, king : &PieceRecord, threatening_pieces : &Vec<PieceRecord>) -> Result<(Vec<PieceRecord>,Vec<PieceRecord>),ChessErrors>{

    let mut checking_pieces : Vec<PieceRecord> = vec![];
    let mut pinning_pieces : Vec<PieceRecord> = vec![];

    // Looking for threats on the friendly king location
    let friendly_king_mask = king.location.binary_location;

    // Look at all threatning piece moves
    for p in threatening_pieces{
        let generated_moves_level_3 = GenerateLevel3Result::from(p, &collision_masks)?;
        for c in generated_moves_level_3.captures{
            if c.binary_location & friendly_king_mask > 0{ // Someone is threatening the king
                checking_pieces.push(*p);
            }else{
                pinning_pieces.push(*p);
            }
        }
    }
    Ok((checking_pieces,pinning_pieces))
}

/// Inspect the provided game state for check, and optionally classify the check.
///
/// This is the primary exported function of the module.
///
/// Parameters:
/// - `game`: the `GameState` to inspect. The function treats this as read-only.
/// - `last_piece_moved_option`: optional `PieceRecord` for the piece that moved
///   on the last ply. When `Some(record)` is supplied the function attempts a
///   full classification of the check (single, discovery, double, checkmate).
///   When `None`, `inspect_check` will only indicate that a check exists and
///   return `UnclassifiedCheck` with the first attacker and the king.
///
/// Return value:
/// - Ok(None) if no checking pieces were found (i.e. the side to move is not in check).
/// - Ok(Some(TypesOfCheck)) when the side to move is in check; the returned
///   `TypesOfCheck` variant gives additional context (see the `types_of_check`
///   module for details).
/// - Err(ChessErrors) if any underlying operation fails (piece-register access,
///   move generation, or simulation errors).
///
/// Behavior:
/// 1. Locate the side-to-move king and collect candidate threatening pieces.
/// 2. Partition candidates into direct checking pieces and non-direct attackers (pins).
/// 3. If there are no checking pieces, return Ok(None).
/// 4. If `last_piece_moved_option` is Some(last_moved):
///    - For every friendly piece, generate all legal-ish replies (level-4),
///      simulate each reply (via `apply_move_to_game_filtering_no_friendly_check`)
///      and check whether any reply eliminates the check. If a legal reply is
///      found, the function returns a classification (Single, Discovery, Double)
///      depending on the number and identity of checking pieces.
///    - If no reply is possible, the function returns `Checkmate`.
/// 5. If `last_piece_moved_option` is None, return `UnclassifiedCheck` with the
///    first discovered attacker and the king.
///
/// Notes and caveats:
/// - The function uses `apply_move_to_game_filtering_no_friendly_check` to
///   simulate moves and ensure they do not leave the mover in check; therefore
///   the classification logic respects legality (not merely pseudo-legal
///   captures).
/// - Double checks are possible and handled explicitly; more than two checking
///   pieces during classification is treated as an error and returned as
///   `ChessErrors::ErrorDuringCheckInspection`.
pub fn inspect_check(game: &GameState, last_piece_moved_option : Option<PieceRecord>) -> Result<Option<TypesOfCheck>,ChessErrors>{
    
    // Look for threats via checks and pins
    let king = game.piece_register.view_king(game.turn)?;
    let threatening_pieces = find_threatening_pieces(&game.piece_register,king)?;
    let collision_masks = CollisionMasks::from(&game.piece_register);
    // Sort into checks or pins
    let (checking_pieces,_pinning_pieces) = sort_threats_to_pins_or_checks(&collision_masks, king, &threatening_pieces)?;
    // If nothing, answer now
    if checking_pieces.len() == 0{
        return Ok(None);
    }


    // Inspection for check / checkmate type

    if let Some(last_moved) = last_piece_moved_option{
    // Look if friendly moves can get out of check  
    let friendly_pieces = match game.turn {
        crate::piece_team::PieceTeam::Light => &game.piece_register.light_pieces,
        crate::piece_team::PieceTeam::Dark => &game.piece_register.dark_pieces
    };
    for (_,p) in friendly_pieces {  
        let generated_moves_level_4 = generate_moves_level_4(
            p,
            &collision_masks,
            &game.special_flags,
            &game.piece_register
        )?;
        // For each move
        for move_to_try in generated_moves_level_4 {
            //dbg!(format!("Move: {:?} in {:?}",move_to_try, game.get_fen()));
            // Simulate the future game, and make sure it doesn't create friendly check
            if let Some(_) = apply_move_to_game_filtering_no_friendly_check(&move_to_try, game)?
            {
                // Found a move that gets out of check
                return match checking_pieces.len(){
                    1 => { // One piece is checking, figure out if it's a discovery check based on the last piece moved and checking piece
                        if checking_pieces[0].location.binary_location == last_moved.location.binary_location{
                            Ok(Some(TypesOfCheck::SingleCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone())))
                        } else {
                            Ok(Some(TypesOfCheck::DiscoveryCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone())))
                        }
                    },
                    2 => Ok(Some(TypesOfCheck::DoubleCheck(*game.piece_register.view_king(game.turn)?,checking_pieces[0].clone(),checking_pieces[1].clone()))),
                    _ => Err(ChessErrors::ErrorDuringCheckInspection("More than two checking pieces found when classifying check type".to_string()))
                };
                
            }
        }
    }
    // No moves found to get out of check, so it's checkmate
    return Ok(Some(TypesOfCheck::Checkmate(*game.piece_register.view_king(game.turn)?, checking_pieces[0].clone())));
    }else{
        Ok(Some(TypesOfCheck::UnclassifiedCheck(*checking_pieces.first().unwrap(), *king)))
    }
}

#[cfg(test)]
/// Unit tests for check inspection and classification.
///
/// These tests exercise `inspect_check` across a set of representative tactical
/// positions constructed via FEN. The tests validate:
/// - Unclassified detection of check when no last-moved piece is supplied.
/// - Correct classification of single-check, discovery, double-check and
///   checkmate when a last-moved piece is provided.
/// - That the high-level inspection logic properly integrates move generation
///   and simulation to distinguish between check and mate.
///
/// The tests intentionally use `unwrap` liberally because they assert on known
/// valid test positions; failures indicate either a logic regression or an
/// unexpected error in auxiliary components.
mod test{
    use super::*;
    use crate::board_location::BoardLocation;
    #[test]
    fn test_inspect_check(){
        let game = GameState::from_fen("rnb1kbnr/ppp1pppp/8/8/4P3/8/PPP2PPP/RNBqKBNR w KQkq - 0 4").unwrap();
        let check_inspection = inspect_check(&game, None).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::UnclassifiedCheck(_,_))));
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d1").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::SingleCheck(_,_))));

        let game = GameState::from_fen("Q4k2/7K/8/8/8/8/8/8 b - - 1 1").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::SingleCheck(_,_))));

        let game = GameState::from_fen("Q5k1/8/6K1/8/8/8/8/8 b - - 1 1").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::Checkmate(_,_))));

        let game = GameState::from_fen("Q4k2/3N4/8/6K1/8/8/8/8 b - - 3 2").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d7").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::DoubleCheck(_,_,_))));

        let game = GameState::from_fen("Q4k2/8/2N5/6K1/8/8/8/8 b - - 3 2").unwrap();
        let check_inspection = inspect_check(&game, Some(*game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("c6").unwrap()).unwrap())).unwrap();
        assert!(matches!(check_inspection,Some(TypesOfCheck::DiscoveryCheck(_,_))));


    }
}