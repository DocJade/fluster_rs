// Writing!

use std::{fs::File, io::Write, os::windows::fs::FileExt};

use crate::pool::disk::{block::block_structs::RawBlock, disk_struct::Disk};

// Add onto the disk type.
// TODO: Only allow writing to allocated blocks.

impl Disk {
    pub fn write_block(&self, block: RawBlock) {
        write_block_direct(&self.disk_file, &block)
    }
}

// Functions
/// DO NOT USE THIS FUNCTION OUTSIDE OF DISK INITIALIZATION
/// USE THE READ METHOD ON YOUR DISKS DIRECTLY.
pub(crate) fn write_block_direct(mut disk_file: &File, block: &RawBlock) {
    // Bounds checking
    if block.block_index.unwrap() >= 2880 {
        // This block is impossible to access.
        // TODO: Error handling
        panic!("Can't write a block past the end of a disk!")
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block.block_index.unwrap() as u64 * 512;

    // Write the data.
    disk_file.seek_write(&block.data, write_offset).unwrap();
    disk_file.flush().unwrap();
}