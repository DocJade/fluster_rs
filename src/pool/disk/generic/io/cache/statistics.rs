// Statistics about the cache

use std::{collections::VecDeque, sync::Mutex};

use lazy_static::lazy_static;

// Holds the cache
const HIT_MEMORY: usize = 10_000; // How many of the last reads we keep track of to calculate hit rate.
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
    hits_and_misses: VecDeque<bool>,
    // How many disk swaps we've prevented
    // swaps_saved: u64
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
    pub(super) fn get_hit_rate() -> f64 {
        // Get ourselves
        let stats = CACHE_STATISTICS.lock().expect("Single threaded");
        if stats.hits_and_misses.is_empty() {
            return 0.0
        }
        // rate is hits / total requests
        let hits = stats.hits_and_misses.iter().filter(|&&hit| hit).count();
        hits as f64 / stats.hits_and_misses.len() as f64
    }
    /// Record a cache hit.
    /// 
    /// Two functions to avoid confusion.
    pub(super) fn record_hit() {
        // Get ourselves
        let stats = &mut CACHE_STATISTICS.lock().expect("Single threaded");

        // Need to pop the oldest hit if we're out of room.
        if stats.hits_and_misses.len() >= HIT_MEMORY {
            let _ = stats.hits_and_misses.pop_back();
        }
        stats.hits_and_misses.push_front(true);
    }

    /// Record a cache hit.
    /// 
    /// Two functions to avoid confusion.
    pub(super) fn record_miss() {
        // Get ourselves
        let stats = &mut CACHE_STATISTICS.lock().expect("Single threaded");

        // Need to pop the oldest hit if we're out of room.
        if stats.hits_and_misses.len() >= HIT_MEMORY {
            let _ = stats.hits_and_misses.pop_back();
        }
        stats.hits_and_misses.push_front(false);
    }
}