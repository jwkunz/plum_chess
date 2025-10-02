use std::error::Error;
use crate::{game_state::GameState, move_logic::*};

fn perft_recursion(game : &GameState, search_depth : u8, current_depth : u8) -> u64{
    let mut count = 0;
    if current_depth == search_depth{
        return 1;
    }
    if let Ok(all_moves) = generate_all_moves(game){
        for m in all_moves{
            if let Ok(next_game) = apply_move_to_game(game, &m.description){
                count += perft_recursion(&next_game, search_depth, current_depth+1);
            }
        }
    }
    count
}


pub fn perft(game : &GameState, search_depth : u8) -> u64{
    perft_recursion(game, search_depth, 0)
}


/// These performance test cases and results are taken from here:
/// https://www.chessprogramming.org/Perft_Results

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn perft_1(){
        let game = GameState::new_game();
        let count = perft(&game, 1);
        assert_eq!(count, 20)
        // Oct 1 Version gave 20 in 0.00s on release  
    }
    #[test]
    fn perft_2(){
        let game = GameState::new_game();
        let count = perft(&game, 2);
        assert_eq!(count, 400)
        // Oct 1 Version gave 400 in 0.00s on release   
    }
    #[test]
    fn perft_3(){
        let game = GameState::new_game();
        let count = perft(&game, 3);
        assert_eq!(count, 8902)
        // Log
        // Oct 1 Version gave 8902 in 0.02s on release            
    }
    #[test]
    fn perft_4(){
        let game = GameState::new_game();
        let count = perft(&game, 4);
        assert_eq!(count, 197281)
        // Log
        // Oct 1 Version gave 197281 in 0.25s on release               
    }
    #[test]
    fn perft_5(){
        let game = GameState::new_game();
        let count = perft(&game, 5);
        assert_eq!(count, 4865609)
        // Log
        // Oct 1 Version gave 4865609 in 6.74s on release      
    }    
    #[test]
    fn perft_6(){
        let game = GameState::new_game();
        let count = perft(&game, 6);
        assert_eq!(count, 119060324)
        // Log
        // Oct 1 Version gave 119060197 in 191.41s on release
    }   
    #[test]
    fn perft_7(){
        let game = GameState::new_game();
        let count = perft(&game, 7);
        assert_eq!(count, 3195901860)
        // Log
    }    
    #[test]
    fn perft_8(){
        let game = GameState::new_game();
        let count = perft(&game, 8);
        assert_eq!(count, 84998978956)
        // Log
    }        
    #[test]
    fn perft_9(){
        let game = GameState::new_game();
        let count = perft(&game, 9);
        assert_eq!(count, 2439530234167	)
        // Log
    }            
}