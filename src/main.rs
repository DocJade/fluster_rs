use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;
use rand::prelude::*;

use crate::disk::disk_structs::Disk;
use crate::helpers::hex_view::hex_view;
use crate::io::read::read_raw_block;
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

    let mut disk_0 = Disk {
        number: 0,
        file: disk_file,
    };

}