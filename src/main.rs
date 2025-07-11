use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;
use rand::prelude::*;

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

    let mut disk_0 = Disk {
        number: 0,
        file: disk_file,
    };

    // burn in test
    let mut rando = rand::rng();
    let mut loops: u128 = 0;
    let mut bytes_written: u128 = 0;
    let mut bytes_incorrect: u128 = 0;
    loop {
        // pick a random block
        let block_index: u16 = rando.random_range(0..2880);

        // now fill it with random data
        let mut data: [u8; 512] = [0u8; 512];
        rando.fill_bytes(&mut data);

        // write that to the disk
        write_raw_block(&mut disk_0, block_index, &data);

        // move to a random position on the disk
        disk_0.file.seek(std::io::SeekFrom::Start(rando.random_range(0..2880*512))).unwrap();

        // now read it back off
        let read_data = read_block(&disk_0, block_index).data;

        // compare the two and count how many bytes were incorrect
        for i in 0..512 {
            bytes_written += 1;
            if data[i] != read_data[i] {
                bytes_incorrect += 1;
            }
        }

        // zero out the block because we are nice.
        write_raw_block(&mut disk_0, block_index, &[0u8; 512]);

        // update loop counter
        loops += 1;

        // print statistics if loops % 10
        if loops % 100 == 0 {
            println!(
                "Loops: {}, Written: {:.6}MB, Incorrect: {}B, Failure Rate: {:.6}%",
                loops,
                bytes_written as f64 / 1024.0 / 1024.0,
                bytes_incorrect,
                (bytes_incorrect as f64 / bytes_written as f64) * 100.0
            );
        }
    }

    // Loops: 41600, Written: 20.312500MB, Incorrect: 1B, Failure Rate: 0.000005%
}