// Writing!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use log::trace;

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

use super::super::block::block_structs::BlockError;
use super::super::block::block_structs::RawBlock;
use std::ops::Rem;
use std::{fs::File, io::Write, os::unix::fs::FileExt};

// Implementations

/// Write a block to the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_block_direct(mut disk_file: &File, block: &RawBlock) -> Result<(), BlockError> {
    trace!(
        "Directly writing block {} to currently inserted disk...",
        block.block_origin.block
    );
    // Bounds checking
    if block.block_origin.block >= 2880 {
        // This block is impossible to access.
        return Err(BlockError::InvalidOffset);
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_origin.block as u64 * 512;

    // Write the data.
    disk_file.write_all_at(&block.data, write_offset)?;
    disk_file.flush()?;
    // The speed difference turning off sync is DRASTIC.
    // Unless we really need this, leaving it off improves speed gains dramatically.
    // disk_file.sync_all()?; 

    trace!("Block written successfully.");
    Ok(())
}

/// Write a vec of bytes starting at offset to the currently inserted disk in the floppy drive.
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_WRITE()!
pub(crate) fn write_large_direct(mut disk_file: &File, data: Vec<u8>, start_block: DiskPointer) -> Result<(), BlockError> {
    // Bounds checking
    assert!(start_block.block <= 2880); // Block is past the end of the disk

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

    // Write the data.
    disk_file.write_all_at(&data, write_offset)?;
    disk_file.flush()?;
    // The speed difference turning off sync is DRASTIC.
    // Unless we really need this, leaving it off improves speed gains dramatically.
    // disk_file.sync_all()?; 

    trace!("Several blocks written successfully.");
    Ok(())
}