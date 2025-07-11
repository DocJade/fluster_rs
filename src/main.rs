use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use crate::helpers::hex_view::hex_view;

mod helpers;




fn main() {
    // Test reading a block on a disk
    // windows raw location of the disk
    let floppy_path_str = r"\\.\A:".to_string();
    let floppy_path = Path::new(&floppy_path_str);

    println!("Opening floppy...");
    let mut raw_disk = File::open(&floppy_path).unwrap();

    // Read in the first block with a buffer
    // Go to the start of the disk
    raw_disk.rewind().unwrap();
    // make a buffer
    let mut read_buffer = [0u8; 512];
    // then read in the disk into the buffer
    raw_disk.read_exact(&mut read_buffer).unwrap();

    // convert to hex, print it out.
    println!("{}", hex_view(read_buffer.to_vec()));

}