// Errors related to interacting with the cache.

#[derive(Debug, Error, PartialEq)]
/// Super-error about the floppy drive itself.
/// 
/// We are unable to handle read errors at this level. All IO related errors
/// are within the DriveIOError type.
pub enum CacheError {
    #[error("No disk is currently in the floppy drive.")]
    DriveEmpty,
    #[error("The operation failed for non-critical reasons, but no corruption occurred, and the operation can be retried with the same arguments.")]
    Retry,
}