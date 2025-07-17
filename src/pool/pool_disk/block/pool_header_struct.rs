// Header for the pool disk
use bitflags::bitflags;
use thiserror::Error;

/// The header of the pool disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PoolHeader {
    /// Flags about the pool.
    pub(super) flags: PoolHeaderFlags,
    /// The highest disk number that we have created
    pub(super) highest_known_disk: u16,
    /// The next disk with a free block on it, or u16::MAX
    pub(super) disk_with_next_free_block: u16,
    /// The number of free blocks across all disks
    pub(super) pool_blocks_free: u16,
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