// Reading!

use std::{fs::File, os::windows::fs::FileExt, path::Path};

// TODO: Disallow unwrap / ensure safety.

// Read a block on the currently inserted disk
pub fn read_block(block: u16) -> [u8; 512] {
    // allocate space for the block
    let mut input_buffer: [u8; 512] = [0u8; 512];

    // Calculate the offset into the disk
    let read_offset: u64 = block as u64 * 512;

    // Open the disk
    let disk = File::open(Path::new(r"\\.\A:")).unwrap();

    // Seek to the requested block and read 512 bytes from it
    disk.seek_read(&mut input_buffer, read_offset).unwrap();

    // send it.
    input_buffer
}