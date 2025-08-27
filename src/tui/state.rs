// Stuff for tracking the state of the TUI and Fluster.
// Need to be careful in here not to lock at the same time
// as Fluster locking other things.

use crate::tui::tasks::ProgressableTask;

/// Struct for holding information about Fluster's current state.
pub(super) struct FlusterTUIState {
    // Stats
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

    // Current in-progress task, if there is one.
    pub(super) task: Option<ProgressableTask>
}