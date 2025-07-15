// Reading!

use std::{fs::File, os::unix::fs::FileExt};

use crate::pool::disk::{block::{block_structs::RawBlock, crc::check_crc}, disk_struct::Disk};

// TODO: Disallow unwrap / ensure safety.
// TODO: Only allow reading allocated blocks.

// Add onto the disk type.

impl Disk {
    pub fn read_block(self, block_index: u16) -> RawBlock {
        read_block_direct(&self.disk_file, block_index)
    }
}

// Private Functions


// Read a block on the currently inserted disk
// TODO: Error handling
/// DO NOT USE THIS FUNCTION OUTSIDE OF DISK INITIALIZATION
/// USE THE READ METHOD ON YOUR DISKS DIRECTLY.
pub(crate) fn read_block_direct(disk_file: &File, block_index: u16) -> RawBlock {

    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        panic!("Can't read a block past the end of a disk!")
    }

    // allocate space for the block
    let mut input_buffer: [u8; 512] = [0u8; 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block_index as u64 * 512;

    // Seek to the requested block and read 512 bytes from it
    disk_file.read_exact_at(&mut input_buffer, read_offset).unwrap();

    // Check the CRC
    assert!(check_crc(input_buffer));

    // send it.
    RawBlock {
        block_index: Some(block_index),
        data: input_buffer,
    }
}