// Header for the pool disk

// Imports
use bitflags::bitflags;
use thiserror::Error;

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
    /// The number of free blocks across all disks
    pub pool_blocks_free: u16,
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PoolHeaderError {
    #[error("Magic was missing, or something else is wrong with the header.")]
    Invalid,
    #[error("Block 0 on this disk is completely blank")]
    Blank,
}
