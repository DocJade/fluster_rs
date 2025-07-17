// I think I slipped a disk.

// Imports

use thiserror::Error;

use crate::pool::disk::generic::block::block_structs::BlockError;
use crate::pool::disk::standard_disk::block::header::header_struct::StandardDiskHeader;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;
use crate::pool::disk::dense_disk::block::header::header_struct::DenseDiskHeader;

// Structs, Enums, Flags


/// The floppy drive
/// The FloppyDrive type doesn't contain anything itself, its just an interface for
/// retrieving the various types of disk.
pub struct FloppyDrive {
    /// Nothing! This type is just for methods.
}

/// The different types of disks contained within a pool.
pub enum DiskType {
    Pool(PoolDisk),
    Standard(StandardDisk),
    Dense(DenseDisk),
    Unknown,
    Blank
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