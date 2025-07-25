// File extents

// Imports

use bitflags::bitflags;
use thiserror::Error;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FileExtent {
    pub(crate) flags: ExtentFlags,
    pub(crate) disk_number: Option<u16>, // not included on local blocks
    /// The block this file's section starts on. Inclusive.
    pub(crate) start_block: u16,
    /// How many blocks in a row starting from the start block
    /// are data blocks for this file.
    /// 
    /// Never traverses disks.
    pub(crate) length: u8,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ExtentFlags: u8 {
        const OnThisDisk = 0b00000010;
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
    }
}

// Error types
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum FileExtentBlockError {
    #[error("There aren't enough free bytes in the block.")]
    NotEnoughSpace,
}
