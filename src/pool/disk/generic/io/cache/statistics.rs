// Statistics about the cache

use std::{collections::VecDeque, sync::Mutex};

use lazy_static::lazy_static;

// Holds the cache
const CACHE_SIZE: usize = 2880; // One floppy worth of blocks.
const HIT_MEMORY: usize = 1000; // How many of the last reads we keep track of to calculate hit rate.
lazy_static! {
    // Where the stats are stored
    static ref CACHE_STATISTICS: Mutex<BlockCacheStatistics> = Mutex::new(BlockCacheStatistics::new());
}

//
// =========
// Structs
// =========
//

/// Statistic information about the cache
pub(super) struct BlockCacheStatistics {
    /// Stats for calculating cache hit rates
    hits_and_misses: VecDeque<bool>, // we will track the last 1000 reads
    // How many disk swaps we've prevented
    //swaps_saved: u64
}

//
// =========
// BlockCacheStatistics functions
// =========
//

// The hit-rate and recoding is public, since its the cache_io that updates and reads these.
impl BlockCacheStatistics {
    /// New stats yay
    fn new() -> Self {
        Self {
            hits_and_misses: VecDeque::with_capacity(HIT_MEMORY),
            // swaps_saved: 0,
        }
    }
    pub(super) fn get_hit_rate() -> f32 {
        // Get ourselves
        let stats = CACHE_STATISTICS.lock().expect("Single threaded");
        if stats.hits_and_misses.is_empty() {
            return 0.0
        }
        // rate is hits / total requests
        let hits = stats.hits_and_misses.iter().filter(|&&hit| hit).count();
        hits as f32 / stats.hits_and_misses.len() as f32
    }
    /// Record a cache hit/miss
    pub(super) fn record_hit(hit: bool) {
        // Get ourselves
        let stats = &mut CACHE_STATISTICS.lock().expect("Single threaded");

        // Need to pop the oldest hit if we're out of room.
        if stats.hits_and_misses.len() >= 1000 {
            stats.hits_and_misses.pop_front();
        }
        stats.hits_and_misses.push_back(hit);
    }
}