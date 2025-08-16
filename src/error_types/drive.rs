// Error types pertaining to the floppy drive itself.
// We do not allow string errors. This is RUST damn it, not python!
use thiserror::Error;

use crate::error_types::critical::CriticalError;

#[derive(Debug, Error, PartialEq)]
/// Super-error about the floppy drive itself.
/// 
/// We are unable to handle read errors at this level. All IO related errors
/// are within the DriveIOError type.
pub enum DriveError {
    #[error("No disk is currently inserted.")]
    DriveEmpty,
    #[error("The operation failed for non-critical reasons, but no corruption occurred, and the operation can be retried with the same arguments.")]
    Retry,
}

#[derive(Debug, Clone, Copy, Error, PartialEq)]
/// Errors related to IO on the inserted floppy disk.
pub enum DriveIOError {
    #[error("No disk is currently inserted.")]
    DriveEmpty,
    #[error("The operation failed for non-critical reasons, but no corruption occurred, and the operation can be retried with the same arguments.")]
    Retry,
}

#[derive(Debug, PartialEq, Clone, Copy)]
/// Reasons why we cannot use the provided floppy disk path
pub enum InvalidDriveReason {
    /// Pointed at a folder instead of a file.
    NotAFile,
    /// We dont have permission to access the path provided
    PermissionDenied,
    /// We do not support using fluster over the network.
    Networking,
    /// Disk must be read and write.
    ReadOnly,
    /// File that refers to the floppy drive is not seekable.
    NotSeekable,
    /// The path is invalid in some way.
    InvalidPath,
    /// The filesystem (or operating system) that you're running fluster on
    /// does not support basic disk IO.
    UnsupportedOS
}