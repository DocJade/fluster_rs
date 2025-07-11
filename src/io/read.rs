// Reading!

use std::{fs::File, os::windows::fs::FileExt, path::Path};
use crate::{block::block_structs::{StructuredBlock, RawBlock, BlockType}, disk::disk_struct::Disk};

// TODO: Disallow unwrap / ensure safety.
// TODO: Make these methods on the disk type (ie disk.read_raw_block())

// Read a block on the currently inserted disk
pub fn read_raw_block(disk: &Disk, block_index: u16) -> RawBlock {

    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        // TODO: Error handling
        panic!("Can't read a block past the end of a disk!")
    }

    // allocate space for the block
    let mut input_buffer: [u8; 512] = [0u8; 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block_index as u64 * 512;

    // Seek to the requested block and read 512 bytes from it
    disk.file.seek_read(&mut input_buffer, read_offset).unwrap();

    // send it.
    RawBlock {
        data: input_buffer,
    }
}