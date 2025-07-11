use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;

use crate::disk::disk_structs::Disk;
use crate::helpers::hex_view::hex_view;
use crate::io::read::read_block;
use crate::io::write::write_raw_block;

mod helpers;
mod io;
mod disk;
mod block;




fn main() {
    // Get the disk handle
    let disk_file: File = OpenOptions::new()
        .read(true)
        .write(true)
        .open(Path::new(r"\\.\A:"))
        .unwrap();

    let disk_0 = Disk {
        number: 0,
        file: disk_file,
    };

    // Read the first block on the disk
    let block_zero = read_block(&disk_0, 0);
    
    // convert to hex, print it out.
    println!("{}", hex_view(block_zero.data.to_vec()));

    // Now we are going to tag this disk
    let mut block_to_write = [0u8; 512];
    let fluster_tag : [u8; 8] = "Fluster!".as_bytes().try_into().unwrap();

    block_to_write[..8].copy_from_slice(&fluster_tag);

    // write the block
    write_raw_block(&disk_0, 0, &block_to_write);

    // read the block again and print it
    let block_zero = read_block(&disk_0, 0);
    println!("{}", hex_view(block_zero.data.to_vec()));

   //  Running `target\debug\fluster_fs.exe`
   //  Offset(h)  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
   // 0000000000  00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
   // 0000000010  00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
   // -- snip --
   // 
   //  Offset(h)  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
   // 0000000000  46 6C 75 73 74 65 72 21 00 00 00 00 00 00 00 00  Fluster!........
   // 0000000010  00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
}