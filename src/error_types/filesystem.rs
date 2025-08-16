use libc::c_int;
use log::error;

use crate::error_types::drive::DriveError;

//
//
// ======
// C Error values
// ======
//
//

// Errors gleamed from
// https://man7.org/linux/man-pages/man3/errno.3.html
// https://man7.org/linux/man-pages/man2/openat.2.html

/// Bro thinks he's Shakespeare.
pub(in super::super) const FILE_NAME_TOO_LONG: c_int = libc::ENAMETOOLONG;
/// Tried to modify a non-empty directory in a way that required it to be empty.
pub(in super::super) const DIRECTORY_NOT_EMPTY: c_int = libc::ENOTEMPTY;
/// This seat's taken.
pub(in super::super) const ITEM_ALREADY_EXISTS: c_int = libc::EEXIST;
/// Tried to do directory stuff to a file.
pub(in super::super) const NOT_A_DIRECTORY: c_int = libc::ENOTDIR;
/// Ad hominem
pub(in super::super) const INVALID_ARGUMENT: c_int = libc::EINVAL;
/// Tried to do things to a directory that it does not support.
pub(in super::super) const IS_A_DIRECTORY: c_int = libc::EISDIR;
/// Function not implemented.
pub(in super::super) const UNIMPLEMENTED: c_int = libc::ENOSYS;
/// This operation is not supported in this filesystem.
pub(in super::super) const UNSUPPORTED: c_int = libc::ENOTSUP; 
/// Access denied / files does not exist.
pub(in super::super) const NO_SUCH_ITEM: c_int = libc::ENOENT;
/// Tried to seek to an invalid file position.
pub(in super::super) const INVALID_SEEK: c_int = libc::ESPIPE;
/// Tried to use a filehandle that is stale. New one is required.
pub(in super::super) const STALE_HANDLE: c_int = libc::ESTALE;
// Generic IO error. The dreaded OS(5) Input/Output error.
pub(in super::super) const GENERIC_FAILURE: c_int = libc::EIO;
/// You are insane.
pub(in super::super) const FILE_TOO_BIG: c_int = libc::EFBIG;
/// Operation was interrupted for some reason, but can be retried.
pub(in super::super) const TRY_AGAIN: c_int = libc::ERESTART;
/// Device / filesystem is busy, try again later.
/// 
/// Should never happen in fluster due to being single threaded.
pub(in super::super) const BUSY: c_int = libc::EBUSY;

impl From<DriveError> for c_int {
    fn from(value: DriveError) -> Self {
        match value {
            DriveError::DriveEmpty => {
                // The drive empty error should never get this high
                error!("Drive empty error should never make it to the filesystem level!");
                error!("Telling file system that we are busy...");
                BUSY
            },
            DriveError::Retry => TRY_AGAIN,
        }
    }
}