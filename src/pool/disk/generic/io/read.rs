// Reading!

// Safety
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

// Imports

use super::super::block::block_structs::BlockError;
use super::super::block::block_structs::RawBlock;
use super::super::block::crc::check_crc;
use std::{fs::File, os::unix::fs::FileExt};

// Implementations

/// Read a block on the currently inserted disk in the floppy drive
/// ONLY FOR LOWER LEVEL USE, USE CHECKED_READ()!
pub(crate) fn read_block_direct(
    disk_file: &File,
    block_index: u16,
    ignore_crc: bool,
) -> Result<RawBlock, BlockError> {
    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        return Err(BlockError::InvalidOffset);
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
    Ok(RawBlock {
        block_index,
        data: read_buffer,
    })
}
