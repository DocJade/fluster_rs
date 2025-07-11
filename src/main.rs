use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;
use rand::prelude::*;

use crate::block::block_structs::RawBlock;
use crate::block::crc::{add_crc_to_block, check_crc, compute_crc};
use crate::disk::disk_struct::Disk;
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

    // // wipe the disk
    // println!("Wiping disk...");
    // for i in 0..2880 {
    //     if i%16 == 0 {
    //         println!("{}%", (i as usize * 100) / 2880)
    //     }
    //     write_raw_block(&mut disk_0, i, &[0u8; 512]);
    // }
    // println!("Done.");
    
    // test the crc
    
    
    let mut block = RawBlock {
        data: [0u8; 512],
    };

    // make some data
    let mut random = rand::rng();
    random.fill(&mut block.data[0..508]);

    // add the crc
    add_crc_to_block(&mut block);

    // write that to the disk
    write_raw_block(&mut disk_0, 0, &block.data);

    // move the head a bit
    disk_0.file.seek(std::io::SeekFrom::Start(0)).unwrap();

    // check the block
    let mut read_block = read_raw_block(&disk_0, 0);
    println!("Normal CRC?: {}",check_crc(&read_block));

    // flip a random bit
    let bit_to_flip: u32 = random.next_u32() % (512*8);

    // flip it in the read block
    // pinpoint the bit
    let byte_to_flip = (bit_to_flip / 8) as usize;
    let bit_in_byte = bit_to_flip % 8;

    // flip it
    read_block.data[byte_to_flip] ^= 1 << bit_in_byte;

    // put it back
    write_raw_block(&mut disk_0, 0, &read_block.data);

    // read it back in again because silly
    disk_0.file.seek(std::io::SeekFrom::Start(0)).unwrap();
    let corrupted_block = read_raw_block(&disk_0, 0);

    // see if we detect it

    println!("Bit flipped CRC?: {}", check_crc(&corrupted_block));

    // Normal CRC?: true
    // Bit flipped CRC?: false

}