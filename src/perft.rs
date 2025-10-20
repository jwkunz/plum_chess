use crate::{chess_errors::ChessErrors, game_state::GameState, generate_moves_level_5::{CheckedMoveWithFutureGame, generate_all_moves}};

#[derive(Debug, PartialEq)]
pub struct PerftCounts{
    pub nodes : usize,
    pub captures : usize,
    pub en_passant : usize,
    pub castles : usize,
    pub promtions : usize,
    pub checks : usize,
    pub discovery_checks : usize,
    pub double_checks : usize,
    pub checkmates : usize,
}
impl PerftCounts {
    fn new() -> Self{
        PerftCounts{
    nodes : 0,
    captures : 0,
    en_passant : 0,
    castles : 0,
    promtions : 0,
    checks : 0,
    discovery_checks : 0,
    double_checks : 0,
    checkmates : 0,
        }
    }
}

fn perft_recursion(state : &CheckedMoveWithFutureGame, search_depth : u8, current_depth : u8, counts : &mut PerftCounts) -> Result<(),ChessErrors>{
    if current_depth == search_depth{
        counts.nodes += 1;
        
        // Handle capture status
        if state.checked_move.description.capture_status.is_some() {
            counts.captures += 1;
        }
        
        // Handle move types
        match state.checked_move.description.move_type {
            crate::move_description::MoveTypes::EnPassant => counts.en_passant += 1,
            crate::move_description::MoveTypes::Castling(_) => counts.castles += 1,
            crate::move_description::MoveTypes::Promote(_) => counts.promtions += 1,
            _ => {}
        }
        
        // Handle check status
        match state.checked_move.check_status {
            Some(crate::types_of_check::TypesOfCheck::UnclassifiedCheck(_,_)) => counts.checks += 1,
            Some(crate::types_of_check::TypesOfCheck::SingleCheck(_,_)) => counts.checks += 1,
            Some(crate::types_of_check::TypesOfCheck::DiscoveryCheck(_,_)) => counts.discovery_checks += 1,
            Some(crate::types_of_check::TypesOfCheck::DoubleCheck(_,_,_)) => counts.double_checks += 1,
            Some(crate::types_of_check::TypesOfCheck::Checkmate(_,_)) => counts.checkmates += 1,
            None => {}
        }                                       
        return Ok(());
    }
    let all_moves = generate_all_moves(&state.game_after_move)?;
    for m in all_moves{
        perft_recursion(&m, search_depth, current_depth+1,counts)?;
    }
    Ok(())
}


pub fn perft(game : &GameState, search_depth : u8) -> Result<PerftCounts,ChessErrors>{
    let mut result = PerftCounts::new();
    let all_moves = generate_all_moves(&game)?;
    for m in all_moves{
        perft_recursion(&m, search_depth, 1,&mut result)?;
    }
    Ok(result)
}


/// These performance test cases and results are taken from here:
/// https://www.chessprogramming.org/Perft_Results

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn perft_position_1(){
        let test_limit = 5;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 20, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 400, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 8902, captures: 34, en_passant: 0, castles: 0, promtions: 0, checks: 12, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 197281, captures: 1576, en_passant: 0, castles: 0, promtions: 0, checks: 469, discovery_checks: 0, double_checks: 0, checkmates: 8 },
            PerftCounts { nodes: 4865609, captures: 82719, en_passant: 258, castles: 0, promtions: 0, checks: 27351, discovery_checks: 6, double_checks: 0, checkmates: 347 },
            PerftCounts { nodes: 119060324, captures: 2812008, en_passant: 5248, castles: 0, promtions: 0, checks: 809099, discovery_checks: 329, double_checks: 46, checkmates: 10828 },
            PerftCounts { nodes: 3195901860, captures: 108329926, en_passant: 319617, castles: 883453, promtions: 0, checks: 33103848, discovery_checks: 18026, double_checks: 1628, checkmates: 435767 },
            PerftCounts { nodes: 84998978956, captures: 3523740106, en_passant: 7187977, castles: 23605205, promtions: 0, checks: 968981593, discovery_checks: 847039, double_checks: 147215, checkmates: 9852036 },
            PerftCounts { nodes: 2439530234167, captures: 125208536153, en_passant: 319496827, castles: 1784356000, promtions: 17334376, checks: 36095901903, discovery_checks: 37101713, double_checks: 5547231, checkmates: 400191963 }
        ];
        let game = GameState::new_game();
        
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            //assert_eq!(count, *target);
        }
        // Oct 1 version [assed up to depth 5 in 7.02 seconds
        // Oct 12 version passed up to depth 5 in 16.78 seconds
        // Oct 19 version ran depth 5 in 9.41 seconds
    }

    #[test]
    fn perft_position_2(){
        let test_limit = 5;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 48, captures: 8, en_passant: 0, castles: 2, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 2039, captures: 351, en_passant: 1, castles: 91, promtions: 0, checks: 3, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 97862, captures: 17102, en_passant: 45, castles: 3162, promtions: 0, checks: 993, discovery_checks: 0, double_checks: 0, checkmates: 1 },
            PerftCounts { nodes: 4085603, captures: 757163, en_passant: 1929, castles: 128013, promtions: 15172, checks: 25523, discovery_checks: 42, double_checks: 6, checkmates: 43 },
            PerftCounts { nodes: 193690690, captures: 35043416, en_passant: 73365, castles: 4993637, promtions: 8392, checks: 3309887, discovery_checks: 19883, double_checks: 2637, checkmates: 30171 },
            PerftCounts { nodes: 8031647685, captures: 1558445089, en_passant: 3577504, castles: 184513607, promtions: 56627920, checks: 92238050, discovery_checks: 568417, double_checks: 54948, checkmates: 360003 }
        ];
        let game = GameState::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0").unwrap();
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            assert_eq!(count, *target);
        }
        // Oct 12 version passed up to depth 2 in 0.24 seconds
    }

    #[test]
    fn perft_position_3(){
        let test_limit = 5;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 14, captures: 1, en_passant: 0, castles: 0, promtions: 0, checks: 2, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 191, captures: 14, en_passant: 0, castles: 0, promtions: 0, checks: 10, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 2812, captures: 209, en_passant: 2, castles: 0, promtions: 0, checks: 267, discovery_checks: 3, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 43238, captures: 3348, en_passant: 123, castles: 0, promtions: 0, checks: 1680, discovery_checks: 106, double_checks: 0, checkmates: 17 },
            PerftCounts { nodes: 674624, captures: 52051, en_passant: 1165, castles: 0, promtions: 0, checks: 52950, discovery_checks: 1292, double_checks: 3, checkmates: 0 },
            PerftCounts { nodes: 11030083, captures: 940350, en_passant: 33325, castles: 0, promtions: 7552, checks: 452473, discovery_checks: 26067, double_checks: 0, checkmates: 2733 },
            PerftCounts { nodes: 178633661, captures: 14519036, en_passant: 294874, castles: 0, promtions: 140024, checks: 12797406, discovery_checks: 370630, double_checks: 3612, checkmates: 87 },
            PerftCounts { nodes: 3009794393, captures: 267586558, en_passant: 8009239, castles: 0, promtions: 6578076, checks: 135626805, discovery_checks: 7181487, double_checks: 1630, checkmates: 450410 }
        ];
        let game = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            assert_eq!(count, *target);
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_4(){
        let test_limit = 2;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 6, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 264, captures: 87, en_passant: 0, castles: 6, promtions: 48, checks: 10, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 9467, captures: 1021, en_passant: 4, castles: 0, promtions: 120, checks: 38, discovery_checks: 0, double_checks: 0, checkmates: 22 },
            PerftCounts { nodes: 422333, captures: 131393, en_passant: 0, castles: 7795, promtions: 60032, checks: 15492, discovery_checks: 0, double_checks: 0, checkmates: 5 },
            PerftCounts { nodes: 15833292, captures: 2046173, en_passant: 6512, castles: 0, promtions: 329464, checks: 200568, discovery_checks: 0, double_checks: 0, checkmates: 50562 },
            PerftCounts { nodes: 706045033, captures: 210369132, en_passant: 212, castles: 10882006, promtions: 81102984, checks: 26973664, discovery_checks: 0, double_checks: 0, checkmates: 81076 }
        ];
        let game = GameState::from_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1").unwrap();
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            assert_eq!(count, *target);
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_5(){
        let test_limit = 2;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 44, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 1486, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 62379, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 2103487, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 89941194, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 }
        ];
        let game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            assert_eq!(count, *target);
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_6(){
        let test_limit = 4;
        let results = vec![
            PerftCounts { nodes: 1, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 46, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 2079, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 89890, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 3894594, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 164075551, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 6923051137, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 287188994746, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 11923589843526, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 },
            PerftCounts { nodes: 490154852788714, captures: 0, en_passant: 0, castles: 0, promtions: 0, checks: 0, discovery_checks: 0, double_checks: 0, checkmates: 0 }
        ];
        let game = GameState::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10").unwrap();
        for (depth,target) in results.iter().enumerate().skip(1).take(test_limit){
            println!("Running Depth: {:}",depth);
            let count = perft(&game, depth as u8).unwrap();
            assert_eq!(count, *target);
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

}