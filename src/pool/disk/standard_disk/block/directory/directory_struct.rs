// Directory struct!

// Imports

use bitflags::bitflags;
use thiserror::Error;

// Structs / Enums / Flags

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) struct DirectoryItem {
    pub(super) flags: DirectoryFlags,
    pub(super) name_length: u8,
    pub(super) name: String,
    pub(super) location: InodeLocation,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) struct DirectoryBlock {
    pub(super) flags: DirectoryBlockFlags,
    pub(super) bytes_free: u16,
    // The disk pointer will automatically deduced from the flags
    pub(super) next_block: u16,
    pub(super) directory_items: Vec<DirectoryItem>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryFlags: u8 {
        const OnThisDisk = 0b00000001;
        const MarkerBit = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryBlockFlags: u8 {
        const FinalDirectoryBlockOnThisDisk = 0b00000001;
    }
}

// Points to a specific inode globally
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InodeLocation {
    pub disk: Option<u16>,
    pub block: u16,
    pub index: u8,
}

// Error types
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum DirectoryBlockError {
    #[error("There aren't enough free bytes in the block.")]
    NotEnoughSpace,
    #[error("Item requested for removal is not present.")]
    NoSuchItem,
}
