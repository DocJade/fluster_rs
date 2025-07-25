// File extents

// Imports

use bitflags::bitflags;
use thiserror::Error;

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
#[derive(Debug, PartialEq, Eq)]
pub struct FileExtentBlock {
    pub(super) flags: FileExtentBlockFlags,
    pub(super) bytes_free: u16,
    pub(super) next_block: FileExtentPointer,
    pub(super) extents: Vec<FileExtent>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileExtentPointer {
    pub(super) disk_number: u16,
    pub(super) block_index: u16,
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
