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
    /// The size of the pointed to file in bytes.
    pub(super) size: u64,
    /// Points to the first extent block in the chain for this file.
    pub(crate) pointer: DiskPointer,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct InodeDirectory {
    /// Points to a DirectoryBlock.
    pub(crate) pointer: DiskPointer,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Relative to Unix Epoch
pub struct InodeTimestamp {
    pub(crate) seconds: u64,
    pub(crate) nanos: u32,
}

// Points to a specific inode globally
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct InodeLocation {
    /// The InodeBlock that this offset goes into can only hold 501 bytes, thus
    /// many of the bits in the u16 are unused. (only 9 out of 16 bits are used).
    /// Thus the upper 7 bits are free for other uses. We can pack flags in here too.
    /// When reconstructing disk pointers, we need to know if this is local or not, so
    /// we will use the second highest bit on the offset to denote if we need a disk number to
    /// reconstruct the pointer.
    /// 
    /// We will use the highest bit as a marker bit for reading.
    /// 
    /// For ordering sake, it'll also go at the front of the type.
    /// 
    /// Must also be private, since you cannot get the usual offset without a method call now.
    pub(super) packed: InodeOffsetPacking,
    /// Disk component automatically gets tossed and added on write/read.
    pub(crate) pointer: DiskPointer,
    /// This offset is extracted from packed during read
    pub(crate) offset: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodeOffsetPacking {
    pub(super) inner: u16, // Combination of flags and the offset.
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeFlags: u8 {
        const FileType = 0b00000001; // Set if this is a file
        const MarkerBit = 0b10000000; // Always set
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(super) struct PackedInodeLocationFlags: u8 {
        const MarkerBit = 0b10000000; // Always set
        const RequiresDisk = 0b01000000; // Does this InodeLocation require a disk to be reconstructed?
        // 5 unused bits.
    }
}

// The block

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InodeBlock {
    pub(super) flags: InodeBlockFlags,
    // Manipulating Inodes must be done through methods on the struct
    pub(super) bytes_free: u16,
    pub(super) next_inode_block: DiskPointer,
    // At runtime its useful to know where this block came from.
    // This doesn't need to get written to disk.
    pub block_origin: DiskPointer, // This MUST be set. it cannot point nowhere.
    pub(super) inodes_data: [u8; 501],
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeBlockFlags: u8 {
        // Currently unused.
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
