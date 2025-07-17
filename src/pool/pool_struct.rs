// Did you know, if lightning struct a pool, everyone dies?
// Imports

use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;


// Structs, Enums, Flags

// All of the information we need about a pool to do our job.
pub struct Pool {
    pub(super) header: PoolDiskHeader,
    /// Pool statistics are not saved to disk, they exist only at runtime.
    pub(super) statistics: PoolStatistics,
}

pub struct PoolStatistics {
    /// How many times we've swapped disks.
    pub(super) swaps: u64,
    /// How many bytes we've read. (Requested by the OS)
    pub(super) data_bytes_read: u64,
    /// Bytes we've read from the disk, including file overhead and such
    pub(super) total_bytes_read: u64,
    /// How many bytes we've written. (Requested by the OS)
    pub(super) data_bytes_written: u64,
    /// Bytes we've read from the disk, including file overhead and such
    pub(super) total_bytes_written: u64,
    /// Rolling cache hit rate.
    pub(super) cache_hit_rate: f32,
}


/// Somebody peed in the pool.
pub enum PoolError {
    SyncFailed,
}