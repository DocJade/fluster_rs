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

#[derive(Debug, Error, PartialEq)]
/// Errors related to IO on the inserted floppy disk.
pub enum DriveIOError {
    #[error("No disk is currently inserted.")]
    DriveEmpty,
    #[error("Parameters given to this IO operation were out of bounds, or otherwise unfulfillable.")]
    Impossible,
    #[error("The operation failed for non-critical reasons, but no corruption occurred, and the operation can be retried with the same arguments.")]
    Retry,
    #[error("An IO operation has failed so hard that we need intervention.")]
    Critical(CriticalError)
}

#[derive(Debug, Error, PartialEq)]
/// Reasons why we cannot use the provided floppy disk path
pub enum InvalidDriveReason {
    /// Pointed at a folder instead of a file.
    NotAFile,
    /// We dont have permission to access the path provided
    PermissionDenied,
}