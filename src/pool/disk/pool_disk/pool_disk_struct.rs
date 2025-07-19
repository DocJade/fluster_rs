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
    /// Map of used blocks on this disk
    pub(super) block_usage_map: [u8; 360],
    // The disk's file
    pub(super) disk_file: std::fs::File,
}
