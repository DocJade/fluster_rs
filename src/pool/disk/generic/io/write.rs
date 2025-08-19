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
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

use super::super::block::block_structs::RawBlock;
use std::ops::Rem;
use std::{
    fs::File,
    os::unix::fs::FileExt
};

// Implementations

/// Write a block to the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_block_direct(disk_file: &File, block: &RawBlock) -> Result<(), DriveError> {
    trace!(
        "Directly writing block {} to currently inserted disk...",
        block.block_origin.block
    );
    // Bounds checking
    if block.block_origin.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  block.block_origin.block)
    }

    let pointer: DiskPointer = block.block_origin;

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_origin.block as u64 * 512;

    for _ in 0..10 {
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
                    return Err(actually_bail)
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Syncing all of the data to the disk is safer.
        // We don't sync if this is a test case, since it makes tests way slower.
        if let Err(failed) = disk_file.sync_all() {
            // That did not work.
            
            // Try converting it into a DriveIOError
            let wrapped: WrappedIOError = WrappedIOError::wrap(failed, pointer);
            let converted: Result<DriveIOError, CannotConvertError> = wrapped.try_into();
            if let Ok(bail) = converted {
                // We don't need to / can't handle this error, up we go.
                // But we might still need to retry this
                if let Ok(actually_bail) = DriveError::try_from(bail) {
                    // Something is up that we cant handle here.
                    return Err(actually_bail)
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Writing worked! all done.
        trace!("Block written successfully.");
        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    error!("Write failure, requires assistance.");

    // Do the error cleanup, if that works, the disk should be working now, and we can recurse, since we
    // should now be able to complete the operation successfully.
    CriticalError::OutOfRetries(RetryCapError::CantWriteBlock).handle();
    write_block_direct(disk_file, block)
}

/// Write a vec of bytes starting at offset to the currently inserted disk in the floppy drive.
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_large_direct(disk_file: &File, data: &Vec<u8>, start_block: DiskPointer) -> Result<(), DriveError> {
    // Bounds checking
    if start_block.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  start_block.block)
    }

    let pointer: DiskPointer = start_block;

    // Must write full blocks (512 byte chunks)
    assert!(data.len().rem(512) == 0);

    // Make sure we don't run off the end of the disk
    assert!(start_block.block + ((data.len().div_ceil(512) - 1) as u16) < 2880_u16);

    trace!(
        "Directly writing {} blocks worth of bytes starting at block {} to currently inserted disk...",
        data.len().div_ceil(512), start_block.block
    );

    // Calculate the offset into the disk
    let write_offset: u64 = start_block.block as u64 * 512;

    // Now enter a loop so we can attempt the write at most 10 times, in case it fails.
    for _ in 0..10 {
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
                    return Err(actually_bail)
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Syncing all of the data to the disk is safer.
        // We don't sync if this is a test case, since it makes tests way slower.
        if let Err(failed) = disk_file.sync_all() {
            // That did not work.
            
            // Try converting it into a DriveIOError
            let wrapped: WrappedIOError = WrappedIOError::wrap(failed, pointer);
            let converted: Result<DriveIOError, CannotConvertError> = wrapped.try_into();
            if let Ok(bail) = converted {
                // We don't need to / can't handle this error, up we go.
                // But we might still need to retry this
                if let Ok(actually_bail) = DriveError::try_from(bail) {
                    // Something is up that we cant handle here.
                    return Err(actually_bail)
                }
            }
            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Writing worked! all done.
        trace!("Several blocks written successfully.");
        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    error!("Write failure, requires assistance.");

    // Do the error cleanup, if that works, the disk should be working now, and we can recurse, since we
    // should now be able to complete the operation successfully.
    CriticalError::OutOfRetries(RetryCapError::CantWriteBlock).handle();
    write_large_direct(disk_file, data, start_block)
}