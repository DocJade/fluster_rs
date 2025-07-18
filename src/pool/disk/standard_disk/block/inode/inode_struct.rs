// Inode layout

// Imports

use bitflags::bitflags;
use thiserror::Error;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct Inode {
    pub(super) flags: InodeFlags,
    pub(super) file: Option<InodeFile>,
    pub(super) directory: Option<InodeDirectory>,
    pub(super) created: InodeTimestamp,
    pub(super) modified: InodeTimestamp,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodeFile {
    pub(super) size: u64,
    pub(super) pointer: DiskPointer // Points to extents
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodeDirectory {
    pub(super) pointer: DiskPointer // Points to directory
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Relative to Unix Epoch
pub(super) struct InodeTimestamp {
    pub(super) seconds: u64,
    pub(super) nanos: u32,
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
pub(super) struct InodeBlock {
    pub(super) flags: InodeBlockFlags,
    // Manipulating Inodes must be done through methods on the struct
    pub(super) bytes_free: u16,
    pub(super) next_inode_block: u16,
    pub(super) inodes_data: [u8; 503]
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