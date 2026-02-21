//! Bucketed transposition table keyed by Zobrist hash (V11).
//!
//! V11 refinements:
//! - 4-way set-associative buckets to reduce collision misses.
//! - Depth/bound/age-aware replacement policy.
//! - Generation aging refreshed on probe hits and stores.

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

#[derive(Debug, Clone, Copy, Default)]
struct TTSlot {
    entry: Option<TTEntry>,
    generation: u8,
}

#[derive(Debug, Clone)]
pub struct TranspositionTable {
    buckets: Vec<[TTSlot; Self::BUCKET_SIZE]>,
    bucket_mask: usize,
    current_generation: u8,
    stats: TTStats,
}

impl TranspositionTable {
    const BUCKET_SIZE: usize = 4;

    pub fn new_with_mb(size_mb: usize) -> Self {
        let bytes = size_mb.max(1) * 1024 * 1024;
        let bucket_size = std::mem::size_of::<[TTSlot; Self::BUCKET_SIZE]>().max(1);
        let raw_bucket_count = (bytes / bucket_size).max(1);
        let bucket_count = raw_bucket_count.next_power_of_two().max(1);
        Self {
            buckets: vec![[TTSlot::default(); Self::BUCKET_SIZE]; bucket_count],
            bucket_mask: bucket_count - 1,
            current_generation: 0,
            stats: TTStats::default(),
        }
    }

    #[inline]
    pub fn new_generation(&mut self) {
        self.current_generation = self.current_generation.wrapping_add(1);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buckets.fill([TTSlot::default(); Self::BUCKET_SIZE]);
        self.current_generation = 0;
        self.stats = TTStats::default();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buckets.len() * Self::BUCKET_SIZE
    }

    #[inline]
    pub fn stats(&self) -> TTStats {
        self.stats
    }

    #[inline]
    fn bucket_idx(&self, key: u64) -> usize {
        (key as usize) & self.bucket_mask
    }

    pub fn probe(&mut self, key: u64) -> Option<TTEntry> {
        self.stats.probes += 1;
        let b = self.bucket_idx(key);
        for slot in &mut self.buckets[b] {
            if let Some(entry) = slot.entry {
                if entry.key == key {
                    self.stats.hits += 1;
                    slot.generation = self.current_generation;
                    return Some(entry);
                }
            }
        }
        None
    }

    pub fn store(&mut self, entry: TTEntry) {
        self.stats.stores += 1;
        let b = self.bucket_idx(entry.key);
        let bucket = &mut self.buckets[b];

        // Same-key replacement remains strictly depth-preferred.
        for slot in &mut *bucket {
            if let Some(existing) = slot.entry {
                if existing.key == entry.key {
                    if entry.depth >= existing.depth {
                        slot.entry = Some(entry);
                        slot.generation = self.current_generation;
                    }
                    return;
                }
            }
        }

        // Empty slot if available.
        if let Some(slot) = bucket.iter_mut().find(|s| s.entry.is_none()) {
            slot.entry = Some(entry);
            slot.generation = self.current_generation;
            return;
        }

        // No empty slot: replace the weakest resident.
        let mut victim_idx = 0usize;
        let mut victim_score = i32::MAX;
        for (i, slot) in bucket.iter().enumerate() {
            let existing = slot.entry.expect("bucket contains full slots here");
            let score = replacement_priority(existing, slot.generation, self.current_generation);
            if score < victim_score {
                victim_score = score;
                victim_idx = i;
            }
        }

        let incoming_score =
            replacement_priority(entry, self.current_generation, self.current_generation);
        // Require incoming to be at least as strong as current victim to avoid thrash.
        if incoming_score >= victim_score {
            bucket[victim_idx].entry = Some(entry);
            bucket[victim_idx].generation = self.current_generation;
        }
    }
}

#[inline]
fn replacement_priority(entry: TTEntry, generation: u8, current_generation: u8) -> i32 {
    let age = current_generation.wrapping_sub(generation) as i32;
    let bound_bonus = match entry.bound {
        Bound::Exact => 8,
        Bound::Lower | Bound::Upper => 4,
    };
    i32::from(entry.depth) * 16 + bound_bonus - age * 3
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
    fn same_key_depth_preferred_replacement() {
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
