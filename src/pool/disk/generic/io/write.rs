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
use crate::error_types::critical::CriticalError;
use crate::error_types::drive::DriveIOError;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

use super::super::block::block_structs::RawBlock;
use std::io::ErrorKind;
use std::ops::Rem;
use std::{
    fs::File,
    os::unix::fs::FileExt
};

// Implementations

/// Write a block to the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_block_direct(disk_file: &File, block: &RawBlock) -> Result<(), DriveIOError> {
    trace!(
        "Directly writing block {} to currently inserted disk...",
        block.block_origin.block
    );
    // Bounds checking
    if block.block_origin.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  block.block_origin.block)
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_origin.block as u64 * 512;

    // Now enter a loop so we can attempt the write at most 10 times, in case it fails.
    let mut most_recent_error: Option<(ErrorKind, Option<i32>)> = None;

    for _ in 0..10 {
        // Write the data.
        let write_result = disk_file.write_all_at(&block.data, write_offset);

        if let Err(error) = write_result {
            // That did not work.

            // Update the most recent error
            most_recent_error = Some((error.kind(), error.raw_os_error()));
            
            // Try converting it into a DriveIOError
            let converted: Result<DriveIOError, CannotConvertError> = error.try_into();
            if let Ok(bail) = converted {
                // We don't need to / can't handle this error, up we go.
                return Err(bail)
            }

            // We must handle the error. Down here that just means trying the write again.
            continue;
        }

        // Syncing all of the data to the disk is safer. But for speed reasons its currently disabled.
        // Unless issues arise with data not hitting the disc correctly, this will remain off.
        // disk_file.sync_all()?; 

        // Writing worked! all done.
        trace!("Block written successfully.");
        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    error!("Write failure, requires assistance.");
    
    // Since we made it out of the loop, the error variable MUST be set.
    let error = most_recent_error.expect("Shouldn't be able to exit the loop without an error.");

    // Do the error cleanup, if that works, we'll tell the caller to retry.
    CriticalError::FloppyWriteFailure(error.0, error.1).handle();
    Err(DriveIOError::Retry)
}

/// Write a vec of bytes starting at offset to the currently inserted disk in the floppy drive.
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_large_direct(disk_file: &File, data: Vec<u8>, start_block: DiskPointer) -> Result<(), DriveIOError> {
    // Bounds checking
    if start_block.block >= 2880 {
        // This block is impossible to access.
        panic!("Impossible write offset `{}`!",  start_block.block)
    }

    // Must write full blocks (512 byte chunks)
    assert!(data.len().rem(512) == 0);

    // Make sure we don't run off the end of the disk
    assert!(start_block.block + (data.len().div_ceil(512) - 1) as u16 <= 2880);

    trace!(
        "Directly writing {} blocks worht of bytes starting at block {} to currently inserted disk...",
        data.len().div_ceil(512), start_block.block
    );

    // Calculate the offset into the disk
    let write_offset: u64 = start_block.block as u64 * 512;

    // Now enter a loop so we can attempt the write at most 10 times, in case it fails.
    let mut most_recent_error: Option<(ErrorKind, Option<i32>)> = None;

    for _ in 0..10 {
        // Write the data.
        let write_result = disk_file.write_all_at(&data, write_offset);

        if let Err(error) = write_result {
            // That did not work.

            // Update the most recent error
            most_recent_error = Some((error.kind(), error.raw_os_error()));
            
            // Try converting it into a DriveIOError
            let converted: Result<DriveIOError, CannotConvertError> = error.try_into();
            if let Ok(bail) = converted {
                // We don't need to / can't handle this error, up we go.
                return Err(bail)
            }

            // We must handle error. Down here that just means trying the write again.
            continue;
        }

        // Syncing all of the data to the disk is safer. But for speed reasons its currently disabled.
        // Unless issues arise with data not hitting the disc correctly, this will remain off.
        // disk_file.sync_all()?; 

        // Writing worked! all done.
        trace!("Several blocks written successfully.");
        return Ok(());
    };

    // We've made it outside of the loop. The error is unrecoverable.
    error!("Write failure, requires assistance.");
    
    // Since we made it out of the loop, the error variable MUST be set.
    let error = most_recent_error.expect("Shouldn't be able to exit the loop without an error.");

    // Do the error cleanup, if that works, we'll tell the caller to retry.
    CriticalError::FloppyWriteFailure(error.0, error.1).handle();
    Err(DriveIOError::Retry)
}