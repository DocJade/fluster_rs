use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use crate::helpers::hex_view::hex_view;
use crate::io::read::read_block;

mod helpers;
mod io;
mod disk;




fn main() {
    // Read the first block on the disk
    let block_zero = read_block(0);
    
    // convert to hex, print it out.
    println!("{}", hex_view(block_zero.to_vec()));
}