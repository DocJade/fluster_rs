// Interacting with the pool

// Imports

use std::process::exit;
use log::error;

// Implementations

impl Pool {
    /// Sync information about the pool to disk
    pub fn sync(&self) -> Result<(), ()> {
        sync(self)
    }
    /// Read in pool information from disk
    pub fn load() -> Self {
        load()
    }
    /// Brand new pools need to run some setup functions to get everything in a ready to use state.
    fn initalize(&self) -> Result<(),()> {
        initalize_pool(self)
    }
}

impl PoolStatistics {
    fn new() -> Self {
        PoolStatistics {
            swaps: 0,
            data_bytes_read: 0,
            total_bytes_read: 0,
            data_bytes_written: 0,
            total_bytes_written: 0,
            cache_hit_rate: 0.0,
        }
    }
}


/// Sync information about the pool to disk
pub(super) fn sync(pool: Pool) -> Result<(), ()> {
    todo!()
}


/// Read in pool information from disk.
/// Will prompt to make new pools if needed.
pub(super) fn load() -> Pool {
    // Read in the header. If this fails, we cannot start the filesystem.
    let header = match PoolHeader::read() {
        Ok(ok) => ok,
        Err(error) => {
            // We cannot start the pool without reading in the header!
            error!("Failed to acquire pool header! {error}");
            println!("Failed to load the pool.");
            println!("Reason: {error}");
            println!("Fluster will now exit.");
            exit(-1);
        },
    };

    Pool {
        header,
        statistics: PoolStatistics::new(),
    }
}

/// Set up stuff for a brand new pool
fn initalize_pool(pool: &Pool) -> Result<(),()> {
    // Things a pool needs:
    // A second disk to start storing inodes on.
    // A root directory.

    // Lets get that second disk going
    todo!()
}

/// Add a new disk to the pool.
fn add_disk(pool: &Pool, disk_type: DiskTypes, disk_number: u16) -> Result<(),()> {

}