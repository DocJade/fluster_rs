// Inode layout
use bitflags::bitflags;

use crate::disk::generic_structs::pointer_struct::DiskPointer;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct Inode {
    pub(super) flags: InodeFlags,
    pub(super) file: Option<InodeFile>,
    pub(super) directory: Option<InodeDirectory>,
    pub(super) timestamp: InodeTimestamp
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
        const FileType = 0b00000001;
        const MarkerBit = 0b10000000; // Always set
    }
}

// The block

#[derive(Debug, PartialEq, Eq)]
pub(super) struct InodeBlock {
    pub(super) flags: InodeBlockflags,
    // Manipulating Inodes must be done through methods on the struct
    pub(super) bytes_free: u16,
    pub(super) next_inode_block: u16,
    pub(super) inodes: Vec<Inode>
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeBlockflags: u8 {
        const FinalInodeBlockOnThisDisk = 0b00000001;
    }
}

// Error types

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InodeBlockError {
    NotEnoughSpace,
}