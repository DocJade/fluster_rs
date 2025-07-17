// Interacting with the pool

use std::process::exit;

use crate::pool::{pool_disk::block::pool_header_struct::PoolHeader, pool_struct::{Pool, PoolStatistics}};
use log::error;

impl Pool {
    /// Sync information about the pool to disk
    pub fn sync(self) -> Result<(), ()> {
        sync(self)
    }
    /// Read in pool information from disk
    pub fn initialize() -> Self {
        initialize()
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


/// Read in pool information from disk
pub(super) fn initialize() -> Pool {
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