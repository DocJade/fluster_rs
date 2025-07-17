// Structs that can be deduced from a block

// Imports
use thiserror::Error;

// Structs, Enums, Flags

/// A raw data block
/// This should only be used internally, interfacing into this should
/// be abstracted away into other types (For example DiskHeader)
pub struct RawBlock {
    /// Which block on the disk this is
    pub block_index: u16,
    /// The block in its entirety.
    pub data: [u8; 512]
}

// Error types for block level operations.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum BlockError {
    #[error("CRC checksum does not match read block data.")]
    InvalidCRC,
    #[error("Attempted to access outside of the bounds of the disk.")]
    InvalidOffset,
    #[error("The host OS denied the operation.")]
    PermissionDenied,
    #[error("A write operation failed or otherwise did not write all of the requested data.")]
    WriteFailure,
    #[error("Operation could not be completed, the floppy drive is busy.")]
    DeviceBusy,
    #[error("Operation was interrupted. Can typically be retried.")]
    Interrupted,
    #[error("Operation was deemed invalid by the OS, either due to methods or arguments.")]
    Invalid,
    #[error("The file/disk we are attempting to access is not there.")]
    NotFound,
    // This is our catch all case, includes things like the user is attempting to use a
    // remote file share as a floppy for some reason.
    #[error("The OS returned an unknown error when attempting to access the file")]
    Unknown(String),

}