use crate::{board_location::{BoardLocation}, chess_errors::ChessErrors, piece_team::PieceTeam};

/// Generates the movement for a pawn that is double stepping

pub fn generate_pawn_double_step_movement(board_location : BoardLocation, team : PieceTeam)->Result<BoardLocation,ChessErrors>{
    let direction : i8 = match team {
        PieceTeam::Dark => -2,
        PieceTeam::Light => 2,
    };
    board_location.generate_moved_location_checked(0,direction)
}

/// Generates the movement for a pawn that is double stepping

pub fn generate_pawn_single_step_movement(board_location : BoardLocation, team : PieceTeam)->Result<BoardLocation,ChessErrors>{
    let direction : i8 = match team {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    };
    board_location.generate_moved_location_checked(0,direction)
}

/// Generates the movement for a pawn that is left capturing

pub fn generate_pawn_capture_movement(board_location : BoardLocation, team : PieceTeam, d_file : i8)->Result<BoardLocation,ChessErrors>{
    let d_rank = match team {
        PieceTeam::Dark => -1,
        PieceTeam::Light => 1,
    };
    board_location.generate_moved_location_checked(d_file,d_rank)
}

/// Generates the movement for a knight 
/// direction is 0 through 7 moving counter-clockwise from east->east->north
pub fn generate_knight_movement(board_location : BoardLocation, direction : u8)->Result<BoardLocation,ChessErrors>{
    let (d_file,d_rank) = match direction {
        0 => (2,1),
        1 => (1,2),  
        2 => (-1,2),
        3 => (-2,1),
        4 => (-2,-1),
        5 => (-1,-2),
        6 => (1,-2),   
        7 => (2,-1),             
        _ => {return Err(ChessErrors::InvalidDirectionSelected(direction))}
    };
    board_location.generate_moved_location_checked(d_file,d_rank)
}

/// Generates the movement for a bishop 
/// direction is 0 through 3 moving counter-clockwise from north east
/// distance is the number of squares along the direction
pub fn generate_bishop_movement(board_location : BoardLocation, direction : u8, distance : u8)->Result<BoardLocation,ChessErrors>{
    let (d_file,d_rank) = match direction {
        0 => (1,1),
        1 => (-1,1),  
        2 => (-1,-1),
        3 => (1,-1),           
        _ => {return Err(ChessErrors::InvalidDirectionSelected(direction))}
    };
    let magnitude = distance as i8;
    board_location.generate_moved_location_checked(d_file*magnitude,d_rank*magnitude)
}

/// Generates the movement for a rook 
/// direction is 0 through 3 moving counter-clockwise from east
/// distance is the number of squares along the direction
pub fn generate_rook_movement(board_location : BoardLocation, direction : u8, distance : u8)->Result<BoardLocation,ChessErrors>{
    let (d_file,d_rank) = match direction {
        0 => (1,0),
        1 => (0,1),  
        2 => (-1,0),
        3 => (0,-1),           
        _ => {return Err(ChessErrors::InvalidDirectionSelected(direction))}
    };
    let magnitude = distance as i8;
    board_location.generate_moved_location_checked(d_file*magnitude,d_rank*magnitude)
}

/// Generates the movement for a king 
/// direction is 0 through 7 moving counter-clockwise from east
pub fn generate_king_movement(board_location : BoardLocation, direction : u8)->Result<BoardLocation,ChessErrors>{
    let (d_file,d_rank) = match direction {
        0 => (1,0),
        1 => (1,1),  
        2 => (0,1),
        3 => (-1,1),
        4 => (-1,0),
        5 => (-1,-1),
        6 => (0,-1),   
        7 => (1,-1),             
        _ => {return Err(ChessErrors::InvalidDirectionSelected(direction))}
    };
    board_location.generate_moved_location_checked(d_file,d_rank)
}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_generate_pawn_double_step_movement(){
        let mask = generate_pawn_double_step_movement(BoardLocation::from_long_algebraic("e2").unwrap(),PieceTeam::Light).unwrap();
        let expected = BoardLocation::from_long_algebraic("e4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);

        let mask = generate_pawn_double_step_movement(BoardLocation::from_long_algebraic("e7").unwrap(),PieceTeam::Dark).unwrap();
        let expected = BoardLocation::from_long_algebraic("e5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);
    }
    #[test]
    fn test_generate_pawn_single_step_movement(){
        let mask = generate_pawn_single_step_movement(BoardLocation::from_long_algebraic("e2").unwrap(),PieceTeam::Light).unwrap();
        let expected = BoardLocation::from_long_algebraic("e3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);

        let mask = generate_pawn_single_step_movement(BoardLocation::from_long_algebraic("e7").unwrap(),PieceTeam::Dark).unwrap();
        let expected = BoardLocation::from_long_algebraic("e6").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);
    }    
    #[test]
    fn test_generate_pawn_capture_movement_movement(){
        let mask = generate_pawn_capture_movement(BoardLocation::from_long_algebraic("e7").unwrap(),PieceTeam::Dark,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f6").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);

        let mask = generate_pawn_capture_movement(BoardLocation::from_long_algebraic("e4").unwrap(),PieceTeam::Light,-1).unwrap();
        let expected = BoardLocation::from_long_algebraic("d5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);        
    }     
    #[test]
    fn test_generate_knight_movement(){
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0).unwrap();
        let expected = BoardLocation::from_long_algebraic("g5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f6").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);     
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),2).unwrap();
        let expected = BoardLocation::from_long_algebraic("d6").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),3).unwrap();
        let expected = BoardLocation::from_long_algebraic("c5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);       
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),4).unwrap();
        let expected = BoardLocation::from_long_algebraic("c3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),5).unwrap();
        let expected = BoardLocation::from_long_algebraic("d2").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);     
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),6).unwrap();
        let expected = BoardLocation::from_long_algebraic("f2").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_knight_movement(BoardLocation::from_long_algebraic("e4").unwrap(),7).unwrap();
        let expected = BoardLocation::from_long_algebraic("g3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);                           
    }           
    #[test]
    fn test_generate_bishop_movement(){
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);      
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,2).unwrap();
        let expected = BoardLocation::from_long_algebraic("g6").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);           
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,5);
        assert!(mask.is_err());     
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),1,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("d5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);      
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),2,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("d3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);            
        let mask = generate_bishop_movement(BoardLocation::from_long_algebraic("e4").unwrap(),3,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);                                         
    }       
    #[test]
    fn test_generate_rook_movement(){
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);      
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,2).unwrap();
        let expected = BoardLocation::from_long_algebraic("g4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);           
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0,5);
        assert!(mask.is_err());          
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),1,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("e5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);      
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),2,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("d4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);            
        let mask = generate_rook_movement(BoardLocation::from_long_algebraic("e4").unwrap(),3,1).unwrap();
        let expected = BoardLocation::from_long_algebraic("e3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);                                         
    }   
    #[test]
    fn test_generate_king_movement(){
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),0).unwrap();
        let expected = BoardLocation::from_long_algebraic("f4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),1).unwrap();
        let expected = BoardLocation::from_long_algebraic("f5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);     
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),2).unwrap();
        let expected = BoardLocation::from_long_algebraic("e5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),3).unwrap();
        let expected = BoardLocation::from_long_algebraic("d5").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);       
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),4).unwrap();
        let expected = BoardLocation::from_long_algebraic("d4").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),5).unwrap();
        let expected = BoardLocation::from_long_algebraic("d3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);     
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),6).unwrap();
        let expected = BoardLocation::from_long_algebraic("e3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);    
        let mask = generate_king_movement(BoardLocation::from_long_algebraic("e4").unwrap(),7).unwrap();
        let expected = BoardLocation::from_long_algebraic("f3").unwrap();
        assert_eq!(mask.binary_location,expected.binary_location);                           
    }            
}