// Interacting with the pool

use crate::pool::disk::disk_struct::{Disk, DiskError};
use crate::pool::pool_struct::PoolInfo;

impl PoolInfo {
    /// Sync information about the pool to disk
    pub fn sync(self) -> Result<(), ()> {
        sync(self)
    }
    /// Read in pool information from disk
    pub fn initialize() -> PoolInfo {
        initialize()
    }
}


/// Sync information about the pool to disk
fn sync(pool: PoolInfo) -> Result<(), ()> {
    todo!()
}






/// Read in pool information from disk
fn initialize() -> PoolInfo {
    todo!()
}