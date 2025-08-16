// File extents

// Imports

use bitflags::bitflags;
use thiserror::Error;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FileExtent {
    /// Callers should never have to care about the flags.
    pub(super) flags: ExtentFlags,
    /// Points to the first block of the extent. Inclusive.
    pub(crate) start_block: DiskPointer,
    /// How many blocks in a row starting from the start block
    /// are data blocks for this file.
    /// 
    /// Never traverses disks.
    pub(crate) length: u8,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ExtentFlags: u8 {
        // While the returned extents will always have their disk number set, at a lower level
        // we save bytes by tossing the disk bytes if the extent is local. The disk number is
        // then reconstructed on read.
        const LocalExtent = 0b00000001;
        const MarkerBit = 0b10000000;
    }
}

// Extents block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileExtentBlock {
    pub(super) flags: FileExtentBlockFlags,
    pub(super) bytes_free: u16,
    pub(crate) next_block: DiskPointer,
    // At runtime its useful to know where this block came from.
    // This doesn't need to get written to disk.
    pub block_origin: DiskPointer, // This MUST be set. it cannot point nowhere.
    pub(super) extents: Vec<FileExtent>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct FileExtentBlockFlags: u8 {
        // Currently unused.
    }
}