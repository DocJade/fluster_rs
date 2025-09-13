// Conversions between all of the lower types.

//
// Imports
//

use std::io::ErrorKind;
use std::time::Duration;
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
use crate::tui::notify::NotifyTui;
use crate::tui::tasks::TaskType;



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

        // Sleep for a tad just in case we're doing a retry
        std::thread::sleep(Duration::from_secs(1));

        // Log where we were trying to do IO at when the error occurred.
        debug!("IO error occured while trying to access disk {} block {}", value.error_origin.disk, value.error_origin.block);
        debug!("Error type: {:#?}", value.io_error);

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
                unreachable!("Networked floppy drive??? Really??? gtfo");
            },
            ErrorKind::BrokenPipe => {
                // What
                // I doubt you could even make fluster start with pipes.
                unreachable!("Broken pipe with fluster, why are you using pipes in the first place???");
            },
            ErrorKind::AlreadyExists => {
                // Fluster does not create files during IO operations, only in backups.
                // Therefore this should not happen.
                // Especially since we always open the backups if they already exist.
                unreachable!("Fluster tried to create a file that already existed somehow. This should be impossible.");
            },
            ErrorKind::WouldBlock => {
                // Fluster does not ask for blocking IO.
                // In theory this can just be retried.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::NotADirectory => {
                // This should never happen, since we always try to write to a file, not a directory.
                unreachable!("Fluster does not open directories, this is impossible.");
            },
            ErrorKind::IsADirectory => {
                // User has passed in a directory for the floppy disk drive instead of a file for it.
                CriticalError::DriveInaccessible(InvalidDriveReason::NotAFile).handle();
                // We cant recover from that, but pretend we can
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::DirectoryNotEmpty => {
                // Fluster does not try to delete directories.
                unreachable!("Fluster does not delete directories, this should be impossible.");
            },
            ErrorKind::ReadOnlyFilesystem => {
                // Cant use fluster on read-only floppy for obvious reasons.
                CriticalError::DriveInaccessible(InvalidDriveReason::ReadOnly).handle();
                // If it was just the write-protect notch, we can recover.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::InvalidInput => {
                // The paramaters given for the IO action were bad, chances are, retrying this wont
                // do anything. We're cooked.
                // But hopefully this shouldn't happen because I'm epic sauce :D
                unreachable!("Invalid input parameters into IO action.")
            },
            ErrorKind::InvalidData => {
                // See above, blah blah blah epic sauce
                unreachable!("Data not valid for the operation showed up in IO action.")
            },
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
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::QuotaExceeded => {
                // Not sure what other quotas other than size are possible, the man page
                // quota(1) doesn't specify any other quota types.
                // Plus, this shouldn't happen for raw IO, right?
                unreachable!("Floppy drives shouldn't have a quota.");
            },
            ErrorKind::FileTooLarge => {
                // Fluster does not use an underlying filesystem.
                // Very funny since the biggest files we deal with are in the low MBs
                unreachable!("Somehow a write was too large, even though we dont use a filesystem directly.");
            },
            ErrorKind::ResourceBusy => {
                // Disk is busy, we can retry though.
                // Force caller to retry.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::ExecutableFileBusy => {
                // If you're somehow running the floppy drive as an executable,
                // you have bigger issues.
                unreachable!("How are you running the floppy drive as an executable?");
            },
            ErrorKind::Deadlock => {
                // File locking deadlock, not much we can do here except try again.
                // Force caller to retry
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::CrossesDevices => {
                // Fluster does not do renames on the floppy disk path.
                unreachable!("Fluster does not support rename the file paths, this should never happen.");
            },
            ErrorKind::TooManyLinks => {
                // We do not create links.
                unreachable!("Fluster does not support links, no idea how we got here.");
            },
            ErrorKind::InvalidFilename => {
                // The path to the disk is invalid somehow.
                CriticalError::DriveInaccessible(InvalidDriveReason::InvalidPath).handle();
                // We cant recover from that, but in case we can, just try again.
                Err(CannotConvertError::MustRetry)
            },
            ErrorKind::ArgumentListTooLong => {
                // Fluster does not call programs
                unreachable!("Fluster wasn't able to call an external program. Wait, we don't do that? Huh?");
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
                // We cant recover from that, so this will never be returned.
                Err(CannotConvertError::MustRetry)
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
                // Nothing we can really do.
                panic!("Please visit https://downloadmoreram.com/ then re-run Fluster.");
            },
            ErrorKind::Other => {
                // "This ErrorKind is not used by the standard library."
                // This is impossible to reach.
                unreachable!("Somehow got an `other` error kind, this is impossible as far as i can tell.");
            },
            _ => {
                // This error is newer than the rust version fluster was originally written for.
                // GLHF!
                
                // Is the floppy drive empty?
                // code: 123,
                // message: "No medium found",
                if let Some(raw) = value.io_error.raw_os_error() && raw == 123_i32 {
                    // No disk is in the drive.
                    // This can happen even if there is a disk in the drive, so we keep
                    // trying.
                    debug!("Is no disk inserted?");
                    // Just keep retrying, if there is an issue with the floppy drive, we need to
                    // eventually end up in the panic handler.

                    // Show user that we're waiting for the drive to spin up
                    // We wait 5 seconds. That's usually fast enough.
                    let handle = NotifyTui::start_task(TaskType::WaitingForDriveSpinUp, 5*5);
                    for _ in 0..5*5 {
                        NotifyTui::complete_task_step(&handle);
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    NotifyTui::finish_task(handle);
                    return Err(CannotConvertError::MustRetry)
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