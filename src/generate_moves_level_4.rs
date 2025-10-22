use std::collections::LinkedList;

use crate::{
    board_location::BoardLocation,
    chess_errors::ChessErrors,
    collision_masks::CollisionMasks,
    special_move_flags::SpecialMoveFlags,
    generate_movements::{generate_pawn_capture_movement, generate_pawn_double_step_movement},
    generate_moves_level_3::GenerateLevel3Result,
    move_description::{MoveDescription, MoveTypes, MoveVector},
    piece_class::PieceClass,
    piece_record::{PieceRecord},
    piece_register::{PieceRegister},
    piece_team::PieceTeam,
};

pub type GenerateLevel4Result = LinkedList<MoveDescription>;

/// Level 4 generates all moves for a piece and incorporate special moves
/// Moves are unchecked; they do not filter what violates rules
pub fn generate_moves_level_4(
    piece: &PieceRecord,
    masks: &CollisionMasks,
    special_flags: &SpecialMoveFlags,
    piece_register: &PieceRegister,
) -> Result<GenerateLevel4Result, ChessErrors> {
    // Store result here
    let mut result = GenerateLevel4Result::new();

    let is_pawn = matches!(piece.class, crate::piece_class::PieceClass::Pawn);
    let is_king = matches!(piece.class, crate::piece_class::PieceClass::King);

    // Get all raw moves
    let regular_moves = GenerateLevel3Result::from(piece, masks)?;

    // First look at all the regular capture moves (more likely to be good)
    for location in regular_moves.captures {
        let capture_status = Some(*piece_register.view_piece_at_location(location)?);
        // But if a pawn, look a promotion could be added first
        if is_pawn {
            let (_, stop_rank) = location.get_file_rank();
            // Look if this could be a promotion and capture (lucky guy lol!)
            let light_stop = (stop_rank == 7) && matches!(piece.team, PieceTeam::Light);
            let dark_stop = (stop_rank == 0) && matches!(piece.team, PieceTeam::Dark);
            if light_stop || dark_stop {
                // Add the promotions
                result.extend(generate_promotions(piece, location, capture_status));
                // Move on to the next move
                continue;
            }
        }
        // With pawn specialness out of the way, now add the regular capture
        result.push_back(MoveDescription {
            vector: MoveVector {
                piece_at_start: *piece,
                destination: location,
            },
            move_type: MoveTypes::Regular,
            capture_status,
        });
    }

    // Now look at all the regular no collision moves
    for location in regular_moves.no_collisions {
        // But if a pawn, look if a double step start or promotion could be added first
        if is_pawn {
            let (_, start_rank) = piece.location.get_file_rank();
            let (_, stop_rank) = location.get_file_rank();

            // Look if this was a start move moving up by 1
            let light_start = (start_rank == 1) && matches!(piece.team, PieceTeam::Light);
            let dark_start = (start_rank == 6) && matches!(piece.team, PieceTeam::Dark);
            let d_rank = stop_rank as i8 - start_rank as i8;
            let moving_up_1 = d_rank.abs() == 1;
            if (light_start || dark_start) && moving_up_1 {
                // It was, consider the double step
                if let Ok(x) = generate_pawn_double_step_movement(piece.location, piece.team) {
                    // If no collision
                    if (x.binary_location & (masks.light_mask | masks.dark_mask)) == 0 {
                        // Add the move
                        result.push_back(MoveDescription {
                            vector: MoveVector {
                                piece_at_start: *piece,
                                destination: x,
                            },
                            move_type: MoveTypes::DoubleStep(location),
                            capture_status: None,
                        });
                    }
                }
            }

            // Look if this could be a promotion
            let light_stop = (stop_rank == 7) && matches!(piece.team, PieceTeam::Light);
            let dark_stop = (stop_rank == 0) && matches!(piece.team, PieceTeam::Dark);
            if light_stop || dark_stop {
                // Add the promotions
                result.extend(generate_promotions(piece, location, None));
                // Move on to the next move
                continue;
            }
        }

        // With pawn specialness out of the way, now add the regular move
        result.push_back(MoveDescription {
            vector: MoveVector {
                piece_at_start: *piece,
                destination: location,
            },
            move_type: MoveTypes::Regular,
            capture_status: None,
        });
    }

    // Now consider en passant
    if is_pawn {
        // If there is an en passant
        if let Some(ep_location) = special_flags.en_passant_location {
            // Look left and right for this piece
            for look_direction in [-1, 1] {
                // To see if the en passant location collides with the capture mask for that direction
                if let Ok(mask) =
                    generate_pawn_capture_movement(piece.location, piece.team, look_direction)
                {
                    // If there is a collision
                    if (mask.binary_location & ep_location.binary_location) > 0 {
                        // Add the en passant move
                        result.push_back(MoveDescription {
                            vector: MoveVector {
                                piece_at_start: *piece,
                                destination: ep_location,
                            },
                            move_type: MoveTypes::EnPassant,
                            capture_status: {
                                // Find the piece behind the en passant location
                                let direction = match piece.team {
                                    PieceTeam::Dark => 1,
                                    PieceTeam::Light => -1,
                                };
                                Some(
                                    *piece_register.view_piece_at_location(
                                        ep_location.generate_moved_location_without_validation(
                                            0, direction,
                                        ),
                                    )?,
                                )
                            },
                        });
                        // Since found first, can break early
                        break;
                    }
                }
            }
        }
    }

    // Now consider the castling
    if is_king {
        if matches!(piece.team,PieceTeam::Light){
            if special_flags.can_castle_queen_light{
                // Make sure area is empty with a collision mask
                let empty_square_mask : u64 = 
                    BoardLocation::from_long_algebraic("b1")?.binary_location | 
                    BoardLocation::from_long_algebraic("c1")?.binary_location | 
                    BoardLocation::from_long_algebraic("d1")?.binary_location;
                // If no collisions
                if (empty_square_mask & (masks.light_mask | masks.dark_mask)) == 0{
                    // Add the castling move
                    result.push_back(MoveDescription {
                        // This is for king
                        vector: MoveVector {
                            piece_at_start: *piece,
                            destination: BoardLocation::from_long_algebraic("c1")?,
                        },
                        move_type: MoveTypes::Castling(
                            // This is for the rook
                            MoveVector{ 
                                piece_at_start: *piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a1")?)?, 
                                destination: BoardLocation::from_long_algebraic("d1")?
                            }
                        ),
                        capture_status: None
                    });
                }
            }
            if special_flags.can_castle_king_light{
                // Make sure area is empty with a collision mask
                let empty_square_mask : u64 = 
                    BoardLocation::from_long_algebraic("f1")?.binary_location | 
                    BoardLocation::from_long_algebraic("g1")?.binary_location;
                // If no collisions
                if (empty_square_mask & (masks.light_mask | masks.dark_mask)) == 0{
                    // Add the castling move
                    result.push_back(MoveDescription {
                        // This is for king
                        vector: MoveVector {
                            piece_at_start: *piece,
                            destination: BoardLocation::from_long_algebraic("g1")?,
                        },
                        move_type: MoveTypes::Castling(
                            // This is for the rook
                            MoveVector{ 
                                piece_at_start: *piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("h1")?)?, 
                                destination: BoardLocation::from_long_algebraic("f1")?
                            }
                        ),
                        capture_status: None
                    });
                }
            }
        }else{ // Dark
            if special_flags.can_castle_queen_dark{
                // Make sure area is empty with a collision mask
                let empty_square_mask : u64 = 
                    BoardLocation::from_long_algebraic("b8")?.binary_location | 
                    BoardLocation::from_long_algebraic("c8")?.binary_location | 
                    BoardLocation::from_long_algebraic("d8")?.binary_location;
                // If no collisions
                if (empty_square_mask & (masks.light_mask | masks.dark_mask)) == 0{
                    // Add the castling move
                    result.push_back(MoveDescription {
                        // This is for king
                        vector: MoveVector {
                            piece_at_start: *piece,
                            destination: BoardLocation::from_long_algebraic("c8")?,
                        },
                        move_type: MoveTypes::Castling(
                            // This is for the rook
                            MoveVector{ 
                                piece_at_start: *piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8")?)?, 
                                destination: BoardLocation::from_long_algebraic("d8")?
                            }
                        ),
                        capture_status: None
                    });
                }
            }
            if special_flags.can_castle_king_dark{
                // Make sure area is empty with a collision mask
                let empty_square_mask : u64 = 
                    BoardLocation::from_long_algebraic("f8")?.binary_location | 
                    BoardLocation::from_long_algebraic("g8")?.binary_location;
                // If no collisions
                if (empty_square_mask & (masks.light_mask | masks.dark_mask)) == 0{
                    // Add the castling move
                    result.push_back(MoveDescription {
                        // This is for king
                        vector: MoveVector {
                            piece_at_start: *piece,
                            destination: BoardLocation::from_long_algebraic("g8")?,
                        },
                        move_type: MoveTypes::Castling(
                            // This is for the rook
                            MoveVector{ 
                                piece_at_start: *piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("h8")?)?, 
                                destination: BoardLocation::from_long_algebraic("f8")?
                            }
                        ),
                        capture_status: None
                    });
                }
            }
        }
    }

    Ok(result)
}

/// Helper function for generating promotions
fn generate_promotions(
    piece: &PieceRecord,
    destination: BoardLocation,
    capture_status: Option<PieceRecord>,
) -> GenerateLevel4Result {
    let mut result = GenerateLevel4Result::new();
    // Queen
    let mut promoted_piece = piece.clone();
    promoted_piece.class = PieceClass::Queen;
    result.push_back(MoveDescription {
        vector: MoveVector {
            piece_at_start: *piece,
            destination,
        },
        move_type: MoveTypes::Promote(promoted_piece),
        capture_status,
    });
    // Rook
    let mut promoted_piece = piece.clone();
    promoted_piece.class = PieceClass::Rook;
    result.push_back(MoveDescription {
        vector: MoveVector {
            piece_at_start: *piece,
            destination,
        },
        move_type: MoveTypes::Promote(promoted_piece),
        capture_status,
    });
    // Bishop
    let mut promoted_piece = piece.clone();
    promoted_piece.class = PieceClass::Bishop;
    result.push_back(MoveDescription {
        vector: MoveVector {
            piece_at_start: *piece,
            destination,
        },
        move_type: MoveTypes::Promote(promoted_piece),
        capture_status,
    });
    // Knight
    let mut promoted_piece = piece.clone();
    promoted_piece.class = PieceClass::Knight;
    result.push_back(MoveDescription {
        vector: MoveVector {
            piece_at_start: *piece,
            destination,
        },
        move_type: MoveTypes::Promote(promoted_piece),
        capture_status,
    });
    result
}


#[cfg(test)]
mod tests {
    use crate::game_state::GameState;

    use super::*;

    #[test]
    fn test_bishop_moves_level_4(){
        let game = GameState::from_fen("rnbqk1nr/p1p2ppp/4p3/3p4/1b1PP3/PpPB1N2/1P3PPP/RNBQK2R w KQkq - 1 7").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d3").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),6);

        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("b4").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),7);
    }

    #[test]
    fn test_knight_moves_level_4(){
        let game = GameState::from_fen("rnbqk1nr/p1p2ppp/4p3/3pN3/1b1PP3/PpPB4/1P3PPP/RNBQK2R b KQkq - 2 7").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e5").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),7);

        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("b8").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),3);
    }

    #[test]
    fn test_rook_moves_level_4(){
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a1").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),6);

        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a8").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),0);
    }

    #[test]
    fn test_queen_moves_level_4(){
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d1").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),7);

        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d8").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),6);
    }

    #[test]
    fn test_king_moves_level_4(){
        // Castling and regular moves
        let game = GameState::from_fen("rnbqk2r/p4ppp/2p1pn2/3pN3/1P1PP3/1pPB4/1P3PPP/RNBQK2R w KQkq - 1 9").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e1").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),4);

        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("e8").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),4); // Remember no check filter yet
    }
    #[test]
    fn test_pawn_moves_level_4(){
        // Regular capture
        let game = GameState::from_fen("rnbqk2r/p4pp1/2p1pn2/3pN3/1P1PP1Pp/2PB4/pP3P1P/1NBQK2R b Kkq g3 0 12").unwrap();
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("d5").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),1);

        // Double promotion 
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("a2").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),8);

        // En passant
        let moves = generate_moves_level_4(game.piece_register.view_piece_at_location(BoardLocation::from_long_algebraic("h4").unwrap()).unwrap(),&CollisionMasks::from(&game.piece_register),&game.special_flags,&game.piece_register).unwrap();
        assert_eq!(moves.len(),2);
    }

}