use crate::{chess_errors::ChessErrors, game_state::GameState, generate_moves_level_5::generate_all_moves};

fn perft_recursion(game : &GameState, search_depth : u8, current_depth : u8) -> Result<u64,ChessErrors>{
    let mut count = 0;
    if current_depth == search_depth{
        return Ok(1);
    }
    let all_moves = generate_all_moves(game)?;
    for m in all_moves{
        count += perft_recursion(&m.game_after_move, search_depth, current_depth+1)?;
    }

    Ok(count)
}


pub fn perft(game : &GameState, search_depth : u8) -> Result<u64,ChessErrors>{
    perft_recursion(game, search_depth, 0)
}


/// These performance test cases and results are taken from here:
/// https://www.chessprogramming.org/Perft_Results

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn perft_position_1(){
        let test_limit = 5;
        let results : Vec<u64> = vec![20,400,8902,197281,4865609,119060324,3195901860,84998978956,2439530234167];
        let game = GameState::new_game();
        
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 1 version [assed up to depth 5 in 7.02 seconds
        // Oct 12 version passed up to depth 5 in 16.78 seconds
    }

    #[test]
    fn perft_position_2(){
        let test_limit = 5;
        let results : Vec<u64> = vec![48,2039,97862,4085603,193690690,8031647685];
        let game = GameState::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0").unwrap();
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 12 version passed up to depth 2 in 0.24 seconds
    }

    #[test]
    fn perft_position_3(){
        let test_limit = 5;
        let results : Vec<u64> = vec![14,191,2812,43238,674624,11030083,178633661,3009794393];
        let game = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_4(){
        let test_limit = 2;
        let results : Vec<u64> = vec![6,264,9467,422333,15833292,706045033];
        let game = GameState::from_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1").unwrap();
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_5(){
        let test_limit = 2;
        let results : Vec<u64> = vec![44,1486,62379,2103487,89941194];
        let game = GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

    #[test]
    fn perft_position_6(){
        let test_limit = 4;
        let results : Vec<u64> = vec![46,2079,89890,3894594,164075551,6923051137,287188994746,11923589843526,490154852788714];
        let game = GameState::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10").unwrap();
        for (depth,target) in results.iter().enumerate().take(test_limit){
            println!("Running Depth: {:}",depth+1);
            let count = perft(&game, (depth as u8)+1).unwrap();
            assert_eq!(count,*target)
        }
        // Oct 12 version passed up to depth 5 in 9.5 seconds
    }

}