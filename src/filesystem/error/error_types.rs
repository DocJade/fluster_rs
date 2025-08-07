use std::process::exit;

use libc::c_int;
use log::error;

use crate::pool::disk::drive_struct::FloppyDriveError;

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

// Mapping between FloppyDriveErrors and these other error types
impl From<FloppyDriveError> for c_int {
    fn from(value: FloppyDriveError) -> Self {
        // A lot of these error types SUCK, pastjade stinky.
        // we will just use generic failures for anything we cannot craft a better error for
        error!("Casting a FloppyDriveError into a c_int, low level stuff has failed!");
        error!("Error: \n\n{value:#?}");
        match value {
            FloppyDriveError::Uninitialized => GENERIC_FAILURE,
            FloppyDriveError::NotBlank => GENERIC_FAILURE,
            FloppyDriveError::WipeFailure => GENERIC_FAILURE,
            FloppyDriveError::WrongDisk => {
                // In theory, the user just inserted the incorrect disk.
                // This error _should_ never make it up this high, but
                // at least theoretically we can retry the entire operation.
                error!("User put in the wrong disk it seems. Try again.");
                TRY_AGAIN
            },
            FloppyDriveError::BadHeader(_) => TRY_AGAIN, // Nothing we can do about it at this level. Maybe try again?
            FloppyDriveError::BlockError(block_error) => match block_error {
                crate::pool::disk::generic::block::block_structs::BlockError::InvalidCRC => {
                    // Block CRC should be checked multiple times on read if it fails. If the error made it up here, that block
                    // is for SURE corrupted or in some invalid state.
                    // But, this error should never get this high.
                    // This is such a mess jeez. TODO: Split apart error types to not throw FloppyDiskError everywhere.
                    // Would be nice to have a FlusterError that would indicate if the operation can be retried or if the
                    // filesystem is for sure in a bad state.
                    // Anyways, nothing we can do this high.
                    error!("CRC on a block failed. Probably a sign of disk/pool corruption. Good luck!");
                    return GENERIC_FAILURE
                },
                crate::pool::disk::generic::block::block_structs::BlockError::InvalidOffset => todo!(),
                crate::pool::disk::generic::block::block_structs::BlockError::PermissionDenied => {
                    // Reading from the floppy disk was denied? ...what?
                    error!("Access to the actual floppy drive failed. This is unrecoverable.");
                    println!("Fluster! Did not have permission to access the floppy drive. Did you provide a valid path to it?");
                    exit(-1);
                },
                crate::pool::disk::generic::block::block_structs::BlockError::WriteFailure => {
                    // Writing to the disk failed either partially or entirely.
                    // In theory, all the blocks it was going to use are now just pointlessly reserved.
                    // There shouldn't be any references to this operation if it failed partially.
                    // You _should_ be able to retry this, but this could go nuts REAL fast.
                    error!("A write failure has occurred, filesystem may have been corrupted slightly.");
                    error!("We will keep going regardless, but we are almost certainly degraded.");
                    TRY_AGAIN
                },
                crate::pool::disk::generic::block::block_structs::BlockError::DeviceBusy => {
                    // There is a dedicated DeviceBusy return type we can use here, although realistically we
                    // should not see this assuming fluster has full control over the drive.
                    error!("We tried to access the floppy drive, but it was marked as busy.");
                    // This could possibly result in corruption due to partial writes in loops.
                    error!("This may have introduced corruption, but we will continue.");
                    BUSY
                },
                crate::pool::disk::generic::block::block_structs::BlockError::Interrupted => {
                    // "can typically be retried" ok pastjade i trust
                    error!("A block level operation was interrupted, but we might be able to try again.");
                    // In theory this could corrupt for same 
                    error!("This may have introduced corruption, but we will continue.");
                    TRY_AGAIN
                },
                crate::pool::disk::generic::block::block_structs::BlockError::Invalid => {
                    // The filesystem said whatever we're doing to the floppy drive was invalid.
                    // Blame it on the caller? lol
                    error!("OS deemed the operation attempted on the floppy as invalid.");
                    error!("We're probably cooked at a fundamental level, but we ball.");
                    error!("We will continue, but who knows what will happen now lmao.");
                    INVALID_ARGUMENT
                },
                crate::pool::disk::generic::block::block_structs::BlockError::NotFound => {
                    // The floppy drive does not exist.
                    error!("We tried to access the floppy drive, but it was not there.");
                    // Still not sure if this can happen when fluster attempts to read disks during
                    // a swap, we'll see.
                    // But we shouldn't have to deal with that this high up.
                    // Lower down we should care about this. If it's been passed up this high, chances are
                    // the mount point for the floppy does not exist.
                    println!("Fluster! Could not access the floppy drive. Did you provide a valid path to it?");
                    exit(-1)
                },
                crate::pool::disk::generic::block::block_structs::BlockError::Unknown(text) => {
                    // All bets are off.
                    error!("An unknown error has occurred, no way to handle this, says only rust program where this regularly happens.");
                    println!("============");
                    println!("{text}");
                    println!("============");
                    println!("Dawg, fluster is cooked. Chances are it was not your fault. stopping.");
                    exit(-1)
                },
            },
        }
    }
}