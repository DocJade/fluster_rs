// Header for the pool disk
use bitflags::bitflags;

/// The header of the pool disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PoolHeader {
    /// Flags about the pool.
    flags: PoolHeaderFlags,
    /// The highest disk number that we have created
    highest_known_disk: u16,
    /// The next disk with a free block on it, or u16::MAX
    disk_with_next_free_block: u16,
    /// The number of free blocks across all disks
    pool_blocks_free: u16,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct PoolHeaderFlags: u8 {
    }
}