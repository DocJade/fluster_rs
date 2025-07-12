use std::process::exit;

use crate::{disk::disk_struct::Disk, helpers::hex_view::hex_view};

mod helpers;
mod disk;
mod block;




fn main() {
    // Open disk 0
    let disk = Disk::open(0).unwrap();
    // hex dump that sucker
    println!("{}",hex_view(disk.read_block(0).data.to_vec()))
}