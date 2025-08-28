// Reading!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use log::{
    error,
    warn
};

use crate::{
    error_types::{
        conversions::CannotConvertError, critical::{
            CriticalError,
            RetryCapError
        },
        drive::{
            DriveError,
            DriveIOError,
            WrappedIOError
        }
    },
    pool::disk::generic::generic_structs::pointer_struct::DiskPointer, tui::notify::NotifyTui
};

use super::super::block::block_structs::RawBlock;
use super::super::block::crc::check_crc;
use std::{
    fs::File,
    os::unix::fs::FileExt
};

// Implementations

// DONT USE THE CACHE DOWN HERE!
// We rely on this call to _actually_ read from the disk, not just parrot back what's in the cache.
// The cache calls this when an item isn't found. Checking again down here is pointless. If it was
// in the cache, we wouldn't be here.


/// Read a block on the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_READ()!
pub(crate) fn read_block_direct(
    disk_file: &File,
    originating_disk: u16,
    block_index: u16,
    ignore_crc: bool,
) -> Result<RawBlock, DriveError> {
    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        panic!("Impossible read offset `{block_index}`!")
    }

    let pointer: DiskPointer = DiskPointer {
        disk: originating_disk,
        block: block_index,
    };

    // allocate space for the block
    let mut read_buffer: [u8; 512] = [0u8; 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block_index as u64 * 512;

    // Enter a loop to retry reading the block 10 times at most.
    // If we try 3 times without success, we are cooked.

    for _ in 0..3 {

        // Seek to the requested block and read 512 bytes from it
        let read_result = disk_file.read_exact_at(&mut read_buffer, read_offset);
        if let Err(error) = read_result {
            // That did not work.
            
            // Try converting it into a DriveIOError
            let wrapped: WrappedIOError = WrappedIOError::wrap(error, pointer);
            let converted: Result<DriveIOError, CannotConvertError> = wrapped.try_into();
            if let Ok(bail) = converted {
                // We don't need to / can't handle this error, up we go.
                // But we might still need to retry this
                if let Ok(actually_bail) = DriveError::try_from(bail) {
                    // Something is up that we cant handle here.
                    // We don't bail on missing disks though, sometimes the drive is just being
                    // a bit silly and needs a few tries to realize the disk is in there.
                    if actually_bail == DriveError::DriveEmpty {
                        // Try again.
                        continue;
                    }

                    return Err(actually_bail)
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Read worked.

        // Check the CRC, unless the user disabled it on this call.
        // CRC checks should only be disabled when absolutely needed, such as
        // when reading in unknown blocks from unknown disks to check headers.
        if !ignore_crc && !check_crc(read_buffer) {
            // CRC check failed, we have to try again.
            warn!("CRC check failed, retrying...");
            continue;
        }

        // Read successful.
        // send it.
        let block_origin: DiskPointer = DiskPointer {
            disk: originating_disk,
            block: block_index,
        };

        // Inform TUI
        NotifyTui::block_read();

        return Ok(RawBlock {
            block_origin,
            data: read_buffer,
        });
    }

    // We've made it out of the loop without a good read. We are doomed.
    error!("Read failure, requires assistance.");

    // Do the error cleanup, if that works, we will recurse, since the call should now work.
    CriticalError::OutOfRetries(RetryCapError::CantReadBlock).handle();
    read_block_direct(disk_file, originating_disk, block_index, ignore_crc)
}
