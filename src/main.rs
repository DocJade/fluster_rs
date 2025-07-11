use crate::{disk::disk_struct::Disk, helpers::hex_view::hex_view};

mod helpers;
mod disk;
mod block;




fn main() {
    // Open the disk
    let disk = Disk::open().unwrap();

    // read in the first block
    let block = disk.read_raw_block(0);

    // hex view
    println!("{}",hex_view(block.data.to_vec()));

    // called `Result::unwrap()` on an `Err` value: Uninitialized
    // which is good
}