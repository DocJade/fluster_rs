// Conversions between all of the lower types.

//
// Imports
//

use std::io::ErrorKind;
use std::process::exit;
use log::error;

use thiserror::Error;
use crate::error_types::critical::CriticalError;
use crate::error_types::drive::DriveError;
use crate::error_types::drive::DriveIOError;
use crate::error_types::drive::InvalidDriveReason;



// Not error type can just be converted upwards willy-nilly, that led to the old
// and horrible FloppyDiskError type which everything ended up returning. Not good.

// We do not allow string errors. This is RUST damn it, not python!

// Not all errors allow From due to expectations that the operations that return this
// type either return a value or fail in a critical way.


// We also have a custom conversion error type, so lower level callers can get more info
// about what they need to do to be able to perform the cast to a higher error type.

#[derive(Debug, Clone, Copy, Error, PartialEq)]
/// Errors related to IO on the inserted floppy disk.
pub enum CannotConvertError {
    #[error("You must retry this operation. If retrying repeatedly fails, throw a Critical error.")]
    MustRetry,
}

//
// Drive errors
//


impl TryFrom<DriveIOError> for DriveError {
    type Error = CannotConvertError;

    fn try_from(value: DriveIOError) -> Result<Self, Self::Error> {
        match value {
            DriveIOError::DriveEmpty => {
                // This can be cast upwards.
                // Lower level callers can't do anything
                // about an empty drive.
                Ok(DriveError::DriveEmpty)
            },
            DriveIOError::Retry => {
                // Operation must be retried, cant cast that up.
                Err(CannotConvertError::MustRetry)
            },
            DriveIOError::Critical(critical_error) => {
                // Critical error must be handled.
                // We are the handler.
                critical_error.attempt_recovery();
                // If that worked, now the calling operation needs to be retried.
                Ok(DriveError::Retry)
            },
        }
    }
}

//
// std::io::Error to DriveIOError
//

impl TryFrom<std::io::Error> for DriveIOError {
    type Error = CannotConvertError;

    fn try_from(value: std::io::Error) -> Result<Self, Self::Error> {
        match value.kind() {
            ErrorKind::NotFound => {
                // The floppy drive path is not there.
                // We cannot recover from that.

                Ok(
                    DriveIOError::Critical(
                        CriticalError::FloppyReadFailure(ErrorKind::NotFound, value.raw_os_error())
                    )
                )
            },
            ErrorKind::PermissionDenied => {
                // Dont have permission to perform IO on the drive.
                // Nothing we can do.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::PermissionDenied)
                    )
                )
            },
            ErrorKind::ConnectionRefused |
            ErrorKind::ConnectionReset |
            ErrorKind::HostUnreachable |
            ErrorKind::NetworkUnreachable |
            ErrorKind::ConnectionAborted |
            ErrorKind::NotConnected |
            ErrorKind::AddrInUse |
            ErrorKind::AddrNotAvailable  |
            ErrorKind::NetworkDown |
            ErrorKind::StaleNetworkFileHandle => {
                // Okay you should not be using fluster over the network dawg.
                // 100% your fault
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::Networking)
                    )
                )
            },
            ErrorKind::BrokenPipe => {
                // What
                error!("Broken pipe with fluster, why are you using pipes in the first place???");
                // I doubt you could even make fluster start with pipes.
                unreachable!()
            },
            ErrorKind::AlreadyExists => {
                // Fluster does not create files, it only opens them.
                unreachable!();
            },
            ErrorKind::WouldBlock => {
                // Fluster does not ask for blocking IO.
                unreachable!();
            },
            ErrorKind::NotADirectory => {
                // This should never happen, since we always try to write to a file, not a directory.
                unreachable!()
            },
            ErrorKind::IsADirectory => {
                // User has passed in a directory for the floppy disk drive instead of a file for it.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::NotAFile)
                    )
                )
            },
            ErrorKind::DirectoryNotEmpty => {
                // Fluster does not try to delete directories.
                unreachable!()
            },
            ErrorKind::ReadOnlyFilesystem => {
                // Cant use fluster on read-only floppy for obvious reasons.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::ReadOnly)
                    )
                )
            },
            ErrorKind::InvalidInput => todo!(),
            ErrorKind::InvalidData => todo!(),
            ErrorKind::TimedOut => todo!(),
            ErrorKind::WriteZero => {
                // Writing a complete bytestream failed.
                // Maybe the operation was canceled and needs to be retried?
                // Not sure if the floppy drive requires minimum write sizes, but 512 aught to be enough.
                Ok(
                    DriveIOError::Retry
                )
            },
            ErrorKind::StorageFull => {
                // Fluster does not use a filesystem when doing writes to the disk.
                // Maybe this could happen when attempting to write past the end of the disk?
                // But we have bounds checking for that.
                unreachable!();
            },
            ErrorKind::NotSeekable => {
                // We must be able to seek files to read and write from them, this is a
                // configuration issue.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::NotSeekable)
                    )
                )
            },
            ErrorKind::QuotaExceeded => {
                // Not sure what other quotas other than size are possible, the man page
                // quota(1) doesn't specify any other quota types.
                // Plus, this shouldn't happen for raw IO, right?
                unreachable!()
            },
            ErrorKind::FileTooLarge => {
                // Fluster does not use an underlying filesystem.
                unreachable!()
            },
            ErrorKind::ResourceBusy => {
                // Disk is busy, we can retry though.
                Ok(
                    DriveIOError::Retry
                )
            },
            ErrorKind::ExecutableFileBusy => {
                // If you're somehow running the floppy drive as an executable,
                // you have bigger issues.
                unreachable!()
            },
            ErrorKind::Deadlock => {
                // File locking deadlock, not much we can do here except try again.
                Ok(
                    DriveIOError::Retry
                )
            },
            ErrorKind::CrossesDevices => {
                // Fluster does not do renames on the floppy disk path.
                unreachable!()
            },
            ErrorKind::TooManyLinks => {
                // We do not create links.
                unreachable!()
            },
            ErrorKind::InvalidFilename => {
                // The path to the disk is invalid somehow.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::InvalidPath)
                    )
                )
            },
            ErrorKind::ArgumentListTooLong => {
                // Fluster does not call programs
                unreachable!()
            },
            ErrorKind::Interrupted => {
                // "Interrupted operations can typically be retried."
                Ok(
                    DriveIOError::Retry
                )
            },
            ErrorKind::Unsupported => {
                // Whatever operation we're trying to do, its not possible.
                // Not really much we can do here either.
                Ok(
                    DriveIOError::Critical(
                        CriticalError::DriveInaccessible(InvalidDriveReason::UnsupportedOS)
                    )
                )
            },
            ErrorKind::UnexpectedEof => {
                // This would happen if we read past the end of the floppy disk,
                // which should be protected by guard conditions.
                // Maybe someone's trying to run fluster with 8" disks?
                // We'll just retry the operation, since this should be guarded anyways.
                Ok(
                    DriveIOError::Retry
                )
            },
            ErrorKind::OutOfMemory => {
                // Bro what
                error!("Please visit https://downloadmoreram.com/ then re-run Fluster.");
                exit(-1);
            },
            ErrorKind::Other => {
                // "This ErrorKind is not used by the standard library."
                unreachable!()
            },
            _ => {
                // This error is newer than the rust version fluster was originally written for.
                // GLHF!
                unreachable!("{value:#?}")
            },
        }
    }
}


//
// Filesystem errors
//

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