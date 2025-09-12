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
    pool::disk::generic::generic_structs::pointer_struct::DiskPointer,
    tui::{
        notify::NotifyTui,
        tasks::TaskType
    }
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
    let handle = NotifyTui::start_task(TaskType::DiskReadBlock, 1);
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
        NotifyTui::complete_task_step(&handle);
        // send it.
        let block_origin: DiskPointer = DiskPointer {
            disk: originating_disk,
            block: block_index,
        };

        // Inform TUI
        NotifyTui::finish_task(handle);
        NotifyTui::block_read(1);

        return Ok(RawBlock {
            block_origin,
            data: read_buffer,
        });
    }

    // We've made it out of the loop without a good read. We are doomed.
    error!("Read failure, requires assistance.");

    // Do the error cleanup, if that works, we will recurse, since the call should now work.
    CriticalError::OutOfRetries(RetryCapError::ReadBlock).handle();
    read_block_direct(disk_file, originating_disk, block_index, ignore_crc)
}


/// Automatically truncate reads if it would go off of the end of the disk.
/// 
/// Returns a Vec of RawBlock. May not be the full length of requested blocks.
pub(crate) fn read_multiple_blocks_direct(
    disk_file: &File,
    originating_disk: u16,
    block_index: u16,
    num_to_read: u16,
) -> Result<Vec<RawBlock>, DriveError> {
    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        panic!("Impossible read offset `{block_index}`!")
    }

    // Figure out how many blocks we can read
    let checked_num_to_read = std::cmp::min(num_to_read, 2880 - block_index);

    // Start the read task.
    let handle = NotifyTui::start_task(TaskType::DiskReadBlock, 1);
    
    // The start point of the read
    let pointer: DiskPointer = DiskPointer {
        disk: originating_disk,
        block: block_index,
    };

    // allocate space for the blocks we want to read
    let mut read_buffer: Vec<u8> = vec![0; checked_num_to_read as usize * 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block_index as u64 * 512;

    // Try to read the entire chunk in.
    // If we try 3 times without success, we are cooked.

    for _ in 0..3 {

        // Seek to the requested block and read as many bytes as we need.
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

        // Read successful.
        NotifyTui::complete_task_step(&handle);

        // Split it back out into blocks
        let mut output_blocks: Vec<RawBlock> = Vec::with_capacity(checked_num_to_read.into());

        let block_chunks = read_buffer.chunks_exact(512);
        for (index, block) in block_chunks.enumerate() {

            // Cast the block slice into a known size, this should always work
            let data = match block.try_into() {
                Ok(ok) => ok,
                Err(_) => unreachable!("How was the chunk size of 512 not 512 bytes?"),
            };

            output_blocks.push(
                RawBlock {
                    block_origin: DiskPointer {
                        disk: originating_disk,
                        block: block_index + index as u16
                    },
                    data
                }
            );
        }

        // Inform TUI
        NotifyTui::finish_task(handle);
        NotifyTui::block_read(checked_num_to_read);

        return Ok(output_blocks);
    }

    // We've made it out of the loop without a good read. We are doomed.
    error!("Read failure, requires assistance.");

    // Do the error cleanup, if that works, we will recurse, since the call should now work.
    CriticalError::OutOfRetries(RetryCapError::ReadBlock).handle();
    read_multiple_blocks_direct(disk_file, originating_disk, block_index, num_to_read)
}