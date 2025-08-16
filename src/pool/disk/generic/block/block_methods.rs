// Stuff for block structs!

use log::warn;

// Imports
use super::block_structs::BlockError;

// Implementations

//
// Error type
//

impl From<std::io::Error> for BlockError {
    fn from(value: std::io::Error) -> Self {
        extract_read_error(value)
    }
}

fn extract_read_error(error: std::io::Error) -> BlockError {
    warn!("Got an error when doing io:");
    warn!("{error:#?}");
    // What happened?
    match error.kind() {
        // Our handling of these errors is made with the following assumptions:
        // The user is using a floppy disk.
        // We are doing direct disk accesses, NOT using a filesystem.
        // Fluster is a single threaded file system.

        // Some of these errors seem possible for disk level accesses, but
        // I'm unsure if they could/would actually occur.
        // Thus they are left as todo.

        // Operations that could happen if the user is using a device other
        // than a floppy drive (for some reason) are not checked for directly,
        // and will just pass directly through into BlockError::Unknown().
        std::io::ErrorKind::NotFound => BlockError::NotFound,
        std::io::ErrorKind::PermissionDenied => BlockError::PermissionDenied,
        std::io::ErrorKind::InvalidInput => BlockError::Invalid,
        std::io::ErrorKind::InvalidData => BlockError::Invalid,
        std::io::ErrorKind::TimedOut => todo!("IO timed out! {}", error.to_string()),
        std::io::ErrorKind::WriteZero => BlockError::WriteFailure,
        std::io::ErrorKind::ResourceBusy => BlockError::DeviceBusy,
        std::io::ErrorKind::Interrupted => BlockError::Interrupted,
        _ => {
            // A disk not being in the drive is an uncatagorized error.
            // Annoyingly there is not a type for that, so we have to extract the error code and check that

            // Os {
            //     code: 123,
            //     kind: Uncategorized,
            //     message: "No medium found",
            // }
            if let Some(os_error) = error.raw_os_error() {
                if os_error == 123 {
                    // This is probably no floppy inserted.
                    return BlockError::NotFound;
                }
            }

            // Otherwise, no idea what this error is.
            BlockError::Unknown(error.to_string())
        },
    }
}


