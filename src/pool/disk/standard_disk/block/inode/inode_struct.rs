// Inode layout

// Imports

use bitflags::bitflags;
use thiserror::Error;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Inode {
    pub flags: InodeFlags,
    pub file: Option<InodeFile>,
    pub directory: Option<InodeDirectory>,
    pub created: InodeTimestamp,
    pub modified: InodeTimestamp,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct InodeFile {
    pub(super) size: u64,
    pub(super) pointer: DiskPointer, // Points to extents
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct InodeDirectory {
    pub(super) pointer: DiskPointer, // Points to directory
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Relative to Unix Epoch
pub struct InodeTimestamp {
    pub(super) seconds: u64,
    pub(super) nanos: u32,
}

// Points to a specific inode globally
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InodeLocation {
    pub disk: Option<u16>,
    pub block: u16,
    pub offset: u16,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeFlags: u8 {
        const FileType = 0b00000001; // Set if this is a file
        const MarkerBit = 0b10000000; // Always set
    }
}

// The block

#[derive(Debug, PartialEq, Eq)]
pub struct InodeBlock {
    pub(super) flags: InodeBlockFlags,
    // Manipulating Inodes must be done through methods on the struct
    pub(super) bytes_free: u16,
    pub(super) next_inode_block: u16,
    pub(super) inodes_data: [u8; 503],
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeBlockFlags: u8 {
        const FinalInodeBlockOnThisDisk = 0b00000001;
    }
}

// Error types
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum InodeBlockError {
    #[error("There aren't enough free bytes in the block.")]
    NotEnoughSpace,
    #[error("There are enough free bytes, but there isn't enough contiguous free space.")]
    BlockIsFragmented,
    #[error("An inode does not start at this location.")]
    InvalidOffset,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum InodeReadError {
    #[error("Attempted to read past the end of the inode data.")]
    ImpossibleOffset,
}
