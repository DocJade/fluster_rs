// poooool

// Imports

use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;

// Structs, Enums, Flags
#[derive(Debug)]
pub struct PoolDisk {
    /// Disk number
    pub number: u16,
    /// The disk's header
    pub header: PoolDiskHeader,
    // The disk's file
    pub(super) disk_file: std::fs::File,
}
