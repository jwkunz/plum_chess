//! Fixed-size transposition table keyed by Zobrist hash.
//!
//! This table uses direct indexing with depth-preferred replacement and
//! generation aging to evict stale entries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u8,
    pub score: i32,
    pub bound: Bound,
    pub best_move: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TTStats {
    pub probes: u64,
    pub hits: u64,
    pub stores: u64,
}

#[derive(Debug, Clone)]
pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    generations: Vec<u8>,
    current_generation: u8,
    stats: TTStats,
}

impl TranspositionTable {
    const AGE_REPLACE_THRESHOLD: u8 = 4;
    const DEPTH_REPLACE_MARGIN: u8 = 2;

    pub fn new_with_mb(size_mb: usize) -> Self {
        let bytes = size_mb.max(1) * 1024 * 1024;
        let entry_size = std::mem::size_of::<Option<TTEntry>>().max(1);
        let count = (bytes / entry_size).max(1);
        Self {
            entries: vec![None; count],
            generations: vec![0; count],
            current_generation: 0,
            stats: TTStats::default(),
        }
    }

    /// Advance TT generation (typically once per iterative-deepening iteration).
    #[inline]
    pub fn new_generation(&mut self) {
        self.current_generation = self.current_generation.wrapping_add(1);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.entries.fill(None);
        self.generations.fill(0);
        self.current_generation = 0;
        self.stats = TTStats::default();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn stats(&self) -> TTStats {
        self.stats
    }

    #[inline]
    fn idx(&self, key: u64) -> usize {
        (key as usize) % self.entries.len()
    }

    pub fn probe(&mut self, key: u64) -> Option<TTEntry> {
        self.stats.probes += 1;
        let idx = self.idx(key);
        let hit = self.entries[idx].filter(|e| e.key == key);
        if hit.is_some() {
            self.stats.hits += 1;
            self.generations[idx] = self.current_generation;
        }
        hit
    }

    pub fn store(&mut self, entry: TTEntry) {
        self.stats.stores += 1;
        let idx = self.idx(entry.key);
        match self.entries[idx] {
            None => {
                self.entries[idx] = Some(entry);
                self.generations[idx] = self.current_generation;
            }
            Some(existing) => {
                let same_key = existing.key == entry.key;
                let age = self.current_generation.wrapping_sub(self.generations[idx]);
                let stale = age >= Self::AGE_REPLACE_THRESHOLD;

                let replace = if same_key {
                    entry.depth >= existing.depth
                } else {
                    stale
                        || entry.depth.saturating_add(Self::DEPTH_REPLACE_MARGIN) >= existing.depth
                };

                if replace {
                    self.entries[idx] = Some(entry);
                    self.generations[idx] = self.current_generation;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bound, TTEntry, TranspositionTable};

    #[test]
    fn store_and_probe_round_trip() {
        let mut tt = TranspositionTable::new_with_mb(1);
        let entry = TTEntry {
            key: 123,
            depth: 5,
            score: 42,
            bound: Bound::Exact,
            best_move: Some(99),
        };
        tt.store(entry);
        let got = tt.probe(123).expect("entry should exist");
        assert_eq!(got.key, entry.key);
        assert_eq!(got.depth, entry.depth);
        assert_eq!(got.score, entry.score);
    }

    #[test]
    fn depth_preferred_replacement() {
        let mut tt = TranspositionTable::new_with_mb(1);
        let key = 555;
        tt.store(TTEntry {
            key,
            depth: 2,
            score: 1,
            bound: Bound::Upper,
            best_move: None,
        });
        tt.store(TTEntry {
            key,
            depth: 1,
            score: 9,
            bound: Bound::Exact,
            best_move: Some(77),
        });
        assert_eq!(tt.probe(key).expect("exists").score, 1);
        tt.store(TTEntry {
            key,
            depth: 6,
            score: 3,
            bound: Bound::Lower,
            best_move: Some(88),
        });
        let got = tt.probe(key).expect("exists");
        assert_eq!(got.depth, 6);
        assert_eq!(got.score, 3);
    }
}
