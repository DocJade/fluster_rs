use std::process::exit;

use crate::{disk::disk_struct::Disk, helpers::hex_view::hex_view};

mod helpers;
mod disk;
mod block;




fn main() {
    // Get a disk
    let disk: Disk;

    let attempt = Disk::open();
    if attempt.is_ok() {
        disk = attempt.unwrap();
    } else {
        // What do we need to do?
        match attempt.err().unwrap() {
            disk::disk_struct::DiskError::Uninitialized => {
                // Ask if we want to initialize the disk
                let want = rprompt::prompt_reply("Do you want to initialize the disk? y/n: ").unwrap().contains("y");
                if want {
                    Disk::initialize(0).unwrap();
                    disk = Disk::open().unwrap();
                } else {
                    exit(0);
                }
            },
            disk::disk_struct::DiskError::NotBlank => todo!(),
            disk::disk_struct::DiskError::BlockError(block_error) => todo!(),
        }
    }

    // read in the first block
    let block = disk.read_block(0);

    // hex view
    println!("{}",hex_view(block.data.to_vec()));

    // Header is written properly.
}