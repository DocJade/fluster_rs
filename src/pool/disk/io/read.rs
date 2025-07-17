// Reading!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::{fs::File, os::unix::fs::FileExt};

use crate::pool::disk::{block::{block_structs::{BlockError, RawBlock}, crc::check_crc}, disk_struct::Disk};

// TODO: Disallow unwrap / ensure safety.
// TODO: Only allow reading allocated blocks.

// Add onto the disk type.

impl Disk {
    /// Reads a block from the currently inserted disk.
    /// Interacting with disks should be done through the pool interface, unless operation is internal to the pool.
    pub fn read_block(&self, block_index: u16, disable_crc: bool) -> Result<RawBlock, BlockError> {
        read_block_direct(&self.disk_file, block_index, disable_crc)
    }
}

// Private Functions


// Read a block on the currently inserted disk
// TODO: Error handling
/// DO NOT USE THIS FUNCTION OUTSIDE OF DISK INITIALIZATION
/// USE THE READ METHOD ON YOUR DISKS DIRECTLY.
pub(crate) fn read_block_direct(disk_file: &File, block_index: u16, ignore_crc: bool) -> Result<RawBlock, BlockError> {

    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        return Err(BlockError::InvalidOffset)
    }

    // allocate space for the block
    let mut read_buffer: [u8; 512] = [0u8; 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block_index as u64 * 512;

    // Seek to the requested block and read 512 bytes from it
    disk_file.read_exact_at(&mut read_buffer, read_offset)?;


    // Check the CRC, unless the user disabled it on this call.
    // CRC checks should only be disabled when absolutely needed, such as
    // when reading in unknown blocks from unknown disks to check headers.
    if !ignore_crc && !check_crc(read_buffer) {
        // CRC check failed, 
        return Err(BlockError::InvalidCRC);
    }

    // send it.
    Ok(
        RawBlock {
            block_index,
            data: read_buffer,
        }
    )
}