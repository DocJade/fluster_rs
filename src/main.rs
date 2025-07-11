use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use crate::disk::disk_structs::Disk;
use crate::helpers::hex_view::hex_view;
use crate::io::read::read_block;

mod helpers;
mod io;
mod disk;
mod block;




fn main() {
    // Get the disk handle
    let disk_file: File = File::open(Path::new(r"\\.\A:")).unwrap();
    let disk_0 = Disk {
        number: 0,
        file: disk_file,
    };

    // Read the first block on the disk
    let block_zero = read_block(&disk_0, 2880);
    
    // convert to hex, print it out.
    println!("{}", hex_view(block_zero.data.to_vec()));
}