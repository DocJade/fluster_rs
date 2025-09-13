// Writing!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use log::{
    trace,
    error
};

use crate::error_types::conversions::CannotConvertError;
use crate::error_types::critical::{CriticalError, RetryCapError};
use crate::error_types::drive::{DriveError, DriveIOError, WrappedIOError};
use crate::filesystem::filesystem_struct::WRITE_BACKUPS;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::tui::notify::NotifyTui;
use crate::tui::tasks::TaskType;

use super::super::block::block_structs::RawBlock;
use std::ops::Rem;
use std::{
    fs::File,
    os::unix::fs::FileExt
};

// Implementations

/// Write a block to the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_block_direct(disk_file: &File, block: &RawBlock, has_recursed: bool) -> Result<(), DriveError> {
    let handle = NotifyTui::start_task(TaskType::DiskWriteBlock, 1);
    trace!(
        "Directly writing block {} to currently inserted disk...",
        block.block_origin.block
    );
    // Bounds checking
    if block.block_origin.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  block.block_origin.block)
    }

    // Update the disk backup
    crate::filesystem::disk_backup::update::update_backup(block);

    let pointer: DiskPointer = block.block_origin;

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_origin.block as u64 * 512;

    for _ in 0..3 {
        // Write the data.
        let write_result = disk_file.write_all_at(&block.data, write_offset);

        if let Err(error) = write_result {
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
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Writing worked! all done.
        NotifyTui::complete_task_step(&handle);
        trace!("Block written successfully.");

        // Attempt to sync the write, we only do this if backups are turned on, since we dont
        // wanna slow down tests.
        if let Some(enabled) = WRITE_BACKUPS.get() {
            if *enabled {
                // if this fails, oh well.
                let _ = disk_file.sync_all();
            }
        }

        // Notify the TUI
        NotifyTui::block_written(1);
        NotifyTui::finish_task(handle);

        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    
    // The recursion failed, so the previous error handling failed. We will bail.
    if has_recursed {
        // Rough.
        return Err(DriveError::Retry);
    }
    error!("Write failure, requires assistance.");

    // Do the error cleanup, if that works, the disk should be working now, and we can recurse, since we
    // should now be able to complete the operation successfully.
    CriticalError::OutOfRetries(RetryCapError::WriteBlock).handle();


    // After recovery, the path to the disk may have changed. This is a little naughty, but
    // we'll re-grab the disk file.
    let re_open = FloppyDrive::open_direct(block.block_origin.disk)?;

    // The type doesn't matter, as long as we get the disk file out.
    // If its blank or unknown, we can still write to it, reading would just be bad.
    let new_file = match re_open {
        crate::pool::disk::drive_struct::DiskType::Pool(pool_disk) => {
            pool_disk.disk_file
        },
        crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => {
            standard_disk.disk_file
        },
        crate::pool::disk::drive_struct::DiskType::Unknown(unknown_disk) => {
            unknown_disk.disk_file
        },
        crate::pool::disk::drive_struct::DiskType::Blank(blank_disk) => {
            blank_disk.disk_file
        },
    };

    // Now recurse.

    write_block_direct(&new_file, block, true)
}

/// Write a vec of bytes starting at offset to the currently inserted disk in the floppy drive.
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_large_direct(disk_file: &File, data: &[u8], start_block: DiskPointer) -> Result<(), DriveError> {
    let handle = NotifyTui::start_task(TaskType::DiskWriteLarge, 1);
    // Bounds checking
    if start_block.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  start_block.block)
    }

    let pointer: DiskPointer = start_block;

    // Must write full blocks (512 byte chunks)
    assert!(data.len().rem(512) == 0, "Large writes must be a multiple of 512!");

    // Make sure we don't run off the end of the disk
    assert!(start_block.block + ((data.len().div_ceil(512) - 1) as u16) < 2880_u16, "Write would go off the end of the disk!");

    trace!(
        "Directly writing {} blocks worth of bytes starting at block {} to currently inserted disk...",
        data.len().div_ceil(512), start_block.block
    );

    // Update the disk backup
    crate::filesystem::disk_backup::update::large_update_backup(start_block, data);

    // Calculate the offset into the disk
    let write_offset: u64 = start_block.block as u64 * 512;

    // Pre-sync the disk just in case its already writing.
    if let Some(enabled) = WRITE_BACKUPS.get() {
        if *enabled {
            // if this fails, oh well.
            let _ = disk_file.sync_all();
        }
    }



    // Now enter a loop so we can attempt the write at most 10 times, in case it fails.
    for _ in 0..3 {
        // Write the data.
        let write_result = disk_file.write_all_at(data, write_offset);

        if let Err(error) = write_result {
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
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Writing worked! all done.
        trace!("Several blocks written successfully.");
        NotifyTui::complete_task_step(&handle);

        // Attempt to sync the write, we only do this if backups are turned on, since we dont
        // wanna slow down tests.
        if let Some(enabled) = WRITE_BACKUPS.get() {
            if *enabled {
                // if this fails, oh well.
                let _ = disk_file.sync_all();
            }
        }

        // Notify the TUI
        NotifyTui::finish_task(handle);
        NotifyTui::block_written((data.len()/512) as u16);

        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    error!("Write failure, requires assistance.");

    // Do the error cleanup, if that works, the disk should be working now, we should now be able to write
    // to the disk, but instead of recursing, we call the fallback to try to be a bit safer with the failure, and
    // to prevent infinite recursion.
    CriticalError::OutOfRetries(RetryCapError::WriteBlock).handle();
    large_write_fallback(disk_file, data, start_block)
}

/// If large writes are continually failing, maybe we'll have better luck with singular writes.
fn large_write_fallback(disk_file: &File, data: &[u8], start_block: DiskPointer) -> Result<(), DriveError> {
    // Extract the vec of data into blocks.

    // Pointer that'll be incremented as we're creating blocks
    let mut new_pointer = start_block;

    // Chunk the data into block sized chunks
    let chunked = data.chunks(512);

    // Now this isn't a great idea, but this is the fallback anyways.
    // If any of the chunks (only the last one could have this happen) is
    // less than 512 bytes in size, we'll pad it to 512 with zeros, which yeah, stuff
    // might absolutely explode, but at least we can attempt to keep going lmao

    // Loop over the chunks and construct the blocks
    let blocks: Vec<RawBlock> = chunked.into_iter().map(|chunk|
        {
            // Pad the slice if needed
            let mut padded: Vec<u8> = Vec::with_capacity(512);
            padded.extend(chunk);
            let difference = 512 - padded.len();
            if difference != 0 {
                // Add zeros for padding
                let padding: Vec<u8> = vec![0; difference];
                padded.extend(padding);
            }

            // Now turn that into a slice
            let sliced: [u8; 512] = if let Ok(got) = padded[0..512].try_into() {
                got
            } else {
                // 512 is not 512 today apparently.
                unreachable!("512 is 512")
            };

            // Make a block
            let blocked: RawBlock = RawBlock {
                block_origin: new_pointer,
                data: sliced,
            };

            // Increment the block pointer
            new_pointer.block += 1;

            // Out goes this block
            blocked
        }
    ).collect();

    // Now that we have all of the blocks, write them to the disk
    for block in blocks {
        // This is the first call, we have not recursed.
        write_block_direct(disk_file, &block, false)?;
    }

    Ok(())
}