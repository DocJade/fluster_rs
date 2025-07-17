// Writing!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use std::{fs::File, io::Write, os::{unix::fs::FileExt}};
use super::super::block::block_structs::BlockError;
use super::super::block::block_structs::RawBlock;

// Implementations


/// DO NOT USE THIS FUNCTION OUTSIDE OF DISK INITIALIZATION
/// USE THE READ METHOD ON YOUR DISKS DIRECTLY.
pub(crate) fn write_block_direct(mut disk_file: &File, block: &RawBlock) -> Result<(), BlockError> {
    // Bounds checking
    if block.block_index >= 2880 {
        // This block is impossible to access.
        return Err(BlockError::InvalidOffset)
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_index as u64 * 512;

    // Write the data.
    disk_file.write_all_at(&block.data, write_offset)?;
    disk_file.flush()?;

    Ok(())
}