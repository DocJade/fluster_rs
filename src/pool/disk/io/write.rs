// Writing!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::{fs::File, io::Write, os::{unix::fs::FileExt}};

use crate::pool::disk::{block::block_structs::{BlockError, RawBlock}, disk_struct::Disk};

// Add onto the disk type.
// TODO: Only allow writing to allocated blocks.
// ^ This will happen at the pool level.

impl Disk {
    /// Writes a block to the current disk.
    /// This will return a block error if the write fails.
    /// Interacting with disks should be done through the pool interface, unless operation is internal to the pool.
    pub fn write_block(&self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, block)
    }
}

// Functions
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