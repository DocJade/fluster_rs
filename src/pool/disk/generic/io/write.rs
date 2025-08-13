// Writing!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use log::trace;

use super::super::block::block_structs::BlockError;
use super::super::block::block_structs::RawBlock;
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
    disk_file.sync_all()?;

    trace!("Block written successfully.");
    Ok(())
}
