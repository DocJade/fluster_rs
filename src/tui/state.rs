// Stuff for tracking the state of the TUI and Fluster.
// Need to be careful in here not to lock at the same time
// as Fluster locking other things.

use crate::tui::tasks::ProgressableTask;

/// Struct for holding information about Fluster's current state.
pub(super) struct FlusterTUIState {
    // Disk stats
    // All stats are from the perspective of the current run. Fluster does not
    // store statistics on shutdown.
    /// How many times total the disk has been swapped
    pub(super) disk_swap_count: u64,
    /// The total number of blocks that have been read from the physical disk, note that
    /// this is seperate from cache reads, since we 
    pub(super) disk_blocks_read: u64,
    /// The total number of blocks that have been written to the physical disk.
    pub(super) disk_blocks_written: u64,

    // Cache stats
    /// The current hit rate of the cache (Only needs to be updated on
    /// disk swap.)
    pub(super) cache_hit_rate: f32,
    /// Number of times we went to read a block, but got it from
    /// the cache instead of hitting the disk
    pub(super) cache_blocks_read: u64,
    /// Number of times we went to write a block, but were able to
    /// temporarily store it in the cache.
    pub(super) cache_blocks_written: u64,
    /// Number of times we've flushed tier 0 of the cache to disk.
    pub(super) cache_flushes: u64,
    /// Number of times we avoided swapping disks by doing a cached operation
    pub(super) cache_swaps_saved: u64,

    // Current in-progress task, if there is one.
    pub(super) task: Option<ProgressableTask>
}

impl FlusterTUIState {
    /// Brand new state, used for init
    pub(super) fn new() -> Self {
        Self {
            disk_swap_count: 0,
            disk_blocks_read: 0,
            disk_blocks_written: 0,
            cache_hit_rate: 0.0,
            cache_blocks_read: 0,
            cache_blocks_written: 0,
            cache_flushes: 0,
            cache_swaps_saved: 0,
            task: None,
        }
    }
}