use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;


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
    println!(" Offset(h)  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F");


    let mut offset = 0;
    while offset != 512 {
        // make the line
        let mut string = String::new();
        // first goes the offset, padded so its 10 characters long
        string.push_str(&format!("{:0>10X}  ", offset));
        // now for all the numbers
        for i in 0..16 {
            let byte = read_buffer[offset + i];
            let byte_component = format!("{:02X} ", byte);
            string.push_str(&byte_component);
        }

        // now for the text version
        string.push(' ');
        for i in 0..16 {
            let byte = read_buffer[offset + i];

            // convert
            let mut character = char::from_u32(byte as u32).unwrap_or('?');
            // unless:
            if byte == 0 {
                character = '.';
            }

            string.push(character);
        }

        // line is done. print it
        println!("{}", string);

        // Now increment the offset
        offset += 16;
    }
}