// I think I slipped a disk.

// Imports

use crate::pool::disk::{
    blank_disk::blank_disk_struct::BlankDisk, unknown_disk::unknown_disk_struct::UnknownDisk,
};
use std::fs::File;

use enum_dispatch::enum_dispatch;
use thiserror::Error;

use crate::pool::disk::{
    dense_disk::dense_disk_struct::DenseDisk,
    generic::block::block_structs::{BlockError, RawBlock},
    pool_disk::pool_disk_struct::PoolDisk,
    standard_disk::standard_disk_struct::StandardDisk,
};

// Structs, Enums, Flags

/// The floppy drive
/// The FloppyDrive type doesn't contain anything itself, its just an interface for
/// retrieving the various types of disk.
pub struct FloppyDrive {
    // Nothing! This type is just for methods.
}

/// The different types of disks contained within a pool.
#[enum_dispatch]
#[derive(Debug)]
pub enum DiskType {
    Pool(PoolDisk),
    Standard(StandardDisk),
    Dense(DenseDisk),
    Unknown(UnknownDisk),
    Blank(BlankDisk),
}

#[derive(Debug, Error, PartialEq)]
/// Types of errors that can happen when converting headers
pub enum HeaderConversionError {
    #[error("This block is not a header.")]
    NotAHeaderBlock,
    #[error("This is a different type of header than the one requested.")]
    WrongHeader,
}

#[derive(Debug, Error, PartialEq)]
/// Generic disk error
pub enum FloppyDriveError {
    #[error("Disk is uninitialized")]
    Uninitialized,
    #[error("Disk is not blank")]
    NotBlank,
    #[error("Wipe failed, disk is in an unknown state.")]
    WipeFailure,
    // we'll put this back in later if we need it.
    // #[error("There isn't a disk inserted")]
    // NoDiskInserted,
    #[error("This is not the disk we want")]
    WrongDisk,
    #[error(transparent)]
    BadHeader(#[from] HeaderConversionError),
    #[error(transparent)]
    BlockError(#[from] BlockError),
}

/// All disk types need to be able to create themselves from a raw block.
/// Or, be able to create themselves from a blank disk.
/// We also need to create fake disks to allow creating disks (confusing eh?)
pub trait DiskBootstrap {
    // TODO: Let disk bootstraps fail.
    /// Create brand new disk.
    /// This takes in a blank floppy disk, and does all the needed setup on the disk,
    /// such as writing the header, and other block setup.
    fn bootstrap(file: File, disk_number: u16) -> Result<Self, FloppyDriveError>
    where
        Self: std::marker::Sized;
    /// Create self from incoming header block and file.
    fn from_header(block: RawBlock, file: File) -> Self;
}
