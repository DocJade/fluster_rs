// Conversions between all of the lower types.

//
// Imports
//

use std::io::ErrorKind;
use std::process::exit;
use log::debug;
use log::error;

use log::warn;
use thiserror::Error;
use crate::error_types::critical::CriticalError;
use crate::error_types::drive::DriveError;
use crate::error_types::drive::DriveIOError;
use crate::error_types::drive::InvalidDriveReason;
use crate::error_types::drive::WrappedIOError;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;



// Not error type can just be converted upwards willy-nilly, that led to the old
// and horrible FloppyDiskError type which everything ended up returning. Not good.

// We do not allow string errors. This is RUST damn it, not python!

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
                // Operation must be retried, cant cast that upwards.
                Err(CannotConvertError::MustRetry)
            },
        }
    }
}

//
// std::io:Error wrapping
//

impl WrappedIOError {
    pub(crate) fn wrap(io_error: std::io::Error, error_origin: DiskPointer) -> Self {
        WrappedIOError {
            io_error,
            error_origin,
        }
    }
}

//
// WrappedIOError to DriveIOError
//

impl TryFrom<WrappedIOError> for DriveIOError {
    type Error = CannotConvertError;

    fn try_from(value: WrappedIOError) -> Result<Self, Self::Error> {
        match value.io_error.kind() {
            ErrorKind::NotFound => {
                // The floppy drive path is not there.
                CriticalError::DriveInaccessible(InvalidDriveReason::NotFound).handle();
                // If handling worked, can retry.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::PermissionDenied => {
                // Dont have permission to perform IO on the drive.
                // Nothing we can do.
                CriticalError::DriveInaccessible(InvalidDriveReason::PermissionDenied).handle();
                // If handling worked, can retry.
                Err(CannotConvertError::MustRetry)
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
                CriticalError::DriveInaccessible(InvalidDriveReason::Networking).handle();
                // We cant recover from that
                unreachable!()
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
                // In theory this can just be retried.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::NotADirectory => {
                // This should never happen, since we always try to write to a file, not a directory.
                unreachable!()
            },
            ErrorKind::IsADirectory => {
                // User has passed in a directory for the floppy disk drive instead of a file for it.
                CriticalError::DriveInaccessible(InvalidDriveReason::NotAFile).handle();
                // We cant recover from that
                unreachable!()
            },
            ErrorKind::DirectoryNotEmpty => {
                // Fluster does not try to delete directories.
                unreachable!()
            },
            ErrorKind::ReadOnlyFilesystem => {
                // Cant use fluster on read-only floppy for obvious reasons.
                CriticalError::DriveInaccessible(InvalidDriveReason::ReadOnly).handle();
                // We cant recover from that
                unreachable!()
            },
            ErrorKind::InvalidInput => todo!(),
            ErrorKind::InvalidData => todo!(),
            ErrorKind::TimedOut => {
                // The IO took too long, we should be able to try again.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::WriteZero => {
                // Writing a complete bytestream failed.
                // Maybe the operation was canceled and needs to be retried?
                // Not sure if the floppy drive requires minimum write sizes, but 512 aught to be enough.

                // We dont cast this up, we make the caller retry the write.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::StorageFull => {
                // Fluster does not use a filesystem when doing writes to the disk.
                // Maybe this could happen when attempting to write past the end of the disk?
                // But we have bounds checking for that.
                warn!("Floppy drive claims to be full, we dont care.");
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::NotSeekable => {
                // We must be able to seek files to read and write from them, this is a
                // configuration issue.
                CriticalError::DriveInaccessible(InvalidDriveReason::NotSeekable).handle();
                // We cant recover from that
                unreachable!()
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
                // Force caller to retry.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::ExecutableFileBusy => {
                // If you're somehow running the floppy drive as an executable,
                // you have bigger issues.
                unreachable!()
            },
            ErrorKind::Deadlock => {
                // File locking deadlock, not much we can do here except try again.
                // Force caller to retry
                Err(CannotConvertError::MustRetry)
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
                CriticalError::DriveInaccessible(InvalidDriveReason::InvalidPath).handle();
                // We cant recover from that
                unreachable!()
            },
            ErrorKind::ArgumentListTooLong => {
                // Fluster does not call programs
                unreachable!()
            },
            ErrorKind::Interrupted => {
                // "Interrupted operations can typically be retried."
                // Force caller to retry
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::Unsupported => {
                // Whatever operation we're trying to do, its not possible.
                // Not really much we can do here either.
                CriticalError::DriveInaccessible(InvalidDriveReason::UnsupportedOS).handle();
                // We cant recover from that
                unreachable!()
            },
            ErrorKind::UnexpectedEof => {
                // This would happen if we read past the end of the floppy disk,
                // which should be protected by guard conditions.
                // Maybe someone's trying to run fluster with 8" disks?
                // We'll just retry the operation, since this should be guarded anyways.
                // Force caller to retry
                Err(CannotConvertError::MustRetry)
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
                
                // Is the floppy drive empty?
                // code: 123,
                // message: "No medium found",
                if value.io_error.raw_os_error().expect("Should get a os error number") == 123_i32 {
                    // No disk is in the drive.
                    // This can happen even if there is a disk in the drive, so we keep
                    // trying.
                    debug!("Is no disk inserted?");
                    // Just keep retrying, if there is an issue with the floppy drive, we need to
                    // eventually end up in the panic handler.
                    return Ok(DriveIOError::DriveEmpty)
                }

                // Well, we'll just pretend we can retry any unknown error...
                warn!("UNKNOWN ERROR KIND:");
                warn!("{value:#?}");
                warn!("Ignoring, pretending we can retry...");
                Ok(DriveIOError::Retry)
            },
        }
    }
}