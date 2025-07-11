// Writing!

use std::os::windows::fs::FileExt;

use crate::disk::disk_structs::Disk;

pub fn write_raw_block(disk: &Disk, block_index: u16, data: &[u8; 512]) {
    // Bounds checking
    if block_index >= 2880 {
        // This block is impossible to access.
        // TODO: Error handling
        panic!("Can't write a block past the end of a disk!")
    }

    // Calculate the offset into the disk
    let write_offset: u64 = block_index as u64 * 512;

    // Write the data.
    disk.file.seek_write(data, write_offset).unwrap();
}