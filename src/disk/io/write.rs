// Writing!

use std::{fs::File, io::Write, os::windows::fs::FileExt};

use crate::disk::disk_struct::Disk;

// Add onto the disk type.

impl Disk {
    pub fn write_block(self, block_index: u16, data: &[u8; 512]) {
        write_block(self.disk_file, block_index, data)
    }
}

// Functions

fn write_block(mut disk_file: File, block_index: u16, data: &[u8; 512]) {
    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        // TODO: Error handling
        panic!("Can't write a block past the end of a disk!")
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block_index as u64 * 512;

    // Write the data.
    disk_file.seek_write(data, write_offset).unwrap();
    disk_file.flush().unwrap();
}