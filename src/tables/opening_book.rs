//! Opening-book support with TSV import compatible with public opening datasets.
//!
//! This module can load opening sequences from a tab-separated file and map
//! them into fast hash-indexed candidate moves keyed by position Zobrist hash.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rand::Rng;

use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_apply::apply_move;
use crate::utils::long_algebraic::long_algebraic_to_move_description;

#[derive(Debug, Clone)]
pub struct BookMove {
    pub move_description: u64,
    pub weight: u32,
}

#[derive(Debug, Clone, Default)]
pub struct OpeningBook {
    by_hash: HashMap<u64, Vec<BookMove>>,
}

impl OpeningBook {
    /// Load opening book from `tables/lichess_openings.tsv` when present,
    /// otherwise fallback to a small embedded default table.
    pub fn load_default() -> Self {
        let candidates = [
            "tables/lichess_openings.tsv",
            "tables/openings.tsv",
            "tables/chess-openings.tsv",
        ];

        for p in candidates {
            if Path::new(p).exists() {
                if let Ok(book) = Self::from_tsv_path(p) {
                    return book;
                }
            }
        }

        Self::from_tsv_str(include_str!("data/opening_book_minimal.tsv")).unwrap_or_default()
    }

    pub fn from_tsv_path(path: &str) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| format!("failed reading {path}: {e}"))?;
        Self::from_tsv_str(&data)
    }

    pub fn from_tsv_str(tsv: &str) -> Result<Self, String> {
        let mut lines = tsv.lines().filter(|line| !line.trim().is_empty());
        let header = lines.next().ok_or("opening TSV is empty")?;
        let columns: Vec<&str> = header.split('\t').collect();

        let mut uci_idx = None;
        let mut moves_idx = None;
        let mut weight_idx = None;

        for (i, name) in columns.iter().enumerate() {
            let lc = name.trim().to_ascii_lowercase();
            if lc == "uci" {
                uci_idx = Some(i);
            } else if lc == "moves" {
                moves_idx = Some(i);
            } else if lc == "weight" || lc == "count" || lc == "plays" {
                weight_idx = Some(i);
            }
        }

        let sequence_idx = uci_idx
            .or(moves_idx)
            .ok_or("opening TSV must contain either a 'uci' or 'moves' tab-separated column")?;

        let mut by_hash_and_move: HashMap<u64, HashMap<u64, u32>> = HashMap::new();

        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            let sequence = fields
                .get(sequence_idx)
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or("missing move sequence in opening TSV row")?;

            let row_weight = weight_idx
                .and_then(|idx| fields.get(idx).copied())
                .and_then(|w| w.trim().parse::<u32>().ok())
                .unwrap_or(1);

            let mut state = GameState::new_game();
            for token in sequence.split_whitespace() {
                let mv = long_algebraic_to_move_description(token, &state).map_err(|e| {
                    format!("failed to parse move '{token}' in opening row '{line}': {e}")
                })?;

                let move_weights = by_hash_and_move.entry(state.zobrist_key).or_default();
                let entry = move_weights.entry(mv).or_insert(0);
                *entry = entry.saturating_add(row_weight.max(1));

                state = apply_move(&state, mv).map_err(|e| {
                    format!("failed to apply move '{token}' in opening row '{line}': {e}")
                })?;
            }
        }

        let by_hash = by_hash_and_move
            .into_iter()
            .map(|(hash, moves)| {
                let mut row = Vec::with_capacity(moves.len());
                for (mv, weight) in moves {
                    row.push(BookMove {
                        move_description: mv,
                        weight,
                    });
                }
                (hash, row)
            })
            .collect();

        Ok(Self { by_hash })
    }

    pub fn moves_for(&self, game_state: &GameState) -> Option<&[BookMove]> {
        self.by_hash
            .get(&game_state.zobrist_key)
            .map(|v| v.as_slice())
    }

    pub fn choose_weighted_move<R: Rng + ?Sized>(
        &self,
        game_state: &GameState,
        rng: &mut R,
    ) -> Option<u64> {
        let moves = self.moves_for(game_state)?;
        if moves.is_empty() {
            return None;
        }

        let total_weight: u64 = moves.iter().map(|m| u64::from(m.weight)).sum();
        if total_weight == 0 {
            return Some(moves[0].move_description);
        }

        let mut pick = rng.random_range(0..total_weight);
        for m in moves {
            let w = u64::from(m.weight);
            if pick < w {
                return Some(m.move_description);
            }
            pick -= w;
        }

        Some(moves[0].move_description)
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::OpeningBook;
    use crate::game_state::game_state::GameState;
    use crate::utils::long_algebraic::move_description_to_long_algebraic;

    #[test]
    fn opening_book_parses_and_indexes_start_position() {
        let tsv =
            "eco\tname\tuci\tweight\nC20\tKing Pawn\te2e4 e7e5\t5\nD00\tQueen Pawn\td2d4 d7d5\t3\n";
        let book = OpeningBook::from_tsv_str(tsv).expect("book should parse");
        let start = GameState::new_game();
        let row = book
            .moves_for(&start)
            .expect("start position should be indexed");
        assert!(!row.is_empty());
    }

    #[test]
    fn opening_book_choose_weighted_move_is_legal_lan() {
        let tsv = "uci\tweight\ne2e4 e7e5\t4\nd2d4 d7d5\t1\n";
        let book = OpeningBook::from_tsv_str(tsv).expect("book should parse");
        let start = GameState::new_game();
        let mut rng = StdRng::seed_from_u64(7);
        let mv = book
            .choose_weighted_move(&start, &mut rng)
            .expect("book should choose");
        let lan = move_description_to_long_algebraic(mv, &start).expect("lan conversion");
        assert!(lan == "e2e4" || lan == "d2d4");
    }
}
