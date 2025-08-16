// Header for the pool disk

// Imports
use bitflags::bitflags;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

/// The header of the pool disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PoolDiskHeader {
    /// Flags about the pool.
    pub flags: PoolHeaderFlags,
    /// The highest disk number that we have created
    pub highest_known_disk: u16,
    /// The next disk with a free block on it, or u16::MAX
    pub disk_with_next_free_block: u16,
    /// The number of free standard blocks across all disks
    pub pool_standard_blocks_free: u16,
    /// The disk with the most recent inode write.
    /// Used for speeding up inode additions.
    pub latest_inode_write: DiskPointer,
    /// Map of used blocks on this disk
    pub block_usage_map: [u8; 360],
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct PoolHeaderFlags: u8 {
        // All Pool headers MUST have this bit set.
        const RequiredHeaderBit = 0b10000000;
    }
}