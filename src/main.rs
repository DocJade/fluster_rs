use std::process::exit;

use crate::{disk::{disk_struct::Disk, pool::pool_struct::PoolInfo}, helpers::hex_view::hex_view};

mod helpers;
mod disk;




fn main() {
    // Load in pool info
    let pool: PoolInfo = PoolInfo::initialize();
    // get the root disk
    let disk: Disk = Disk::prompt_for_disk(0).unwrap();
    // wipe it
    disk.wipe();
    println!("Wiped disk.");
}