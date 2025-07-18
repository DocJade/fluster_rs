// I think I slipped a disk.

// Imports

use thiserror::Error;

use crate::pool::disk::{dense_disk::dense_disk_struct::DenseDisk, generic::block::block_structs::{BlockError, RawBlock}, pool_disk::pool_disk_struct::PoolDisk, standard_disk::standard_disk_struct::StandardDisk};


// Structs, Enums, Flags


/// The floppy drive
/// The FloppyDrive type doesn't contain anything itself, its just an interface for
/// retrieving the various types of disk.
pub struct FloppyDrive {
    // Nothing! This type is just for methods.
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

/// All disk types need to be able to create themselves from a raw block.
/// Or, be able to create themselves from a blank disk.
pub trait DiskBootstrap {
    // TODO: Let disk bootstraps fail.
    /// Create brand new disk.
    fn bootstrap(block: RawBlock) -> Self;
    /// Create self from incoming header block.
    fn from_header(block: RawBlock) -> Self;
}