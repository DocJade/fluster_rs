// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]
use rand::{rngs::ThreadRng, Rng, RngCore};
use test_log::test;

use crate::pool::{disk::standard_disk::block::io::directory::tests::get_filesystem, pool_actions::pool_struct::Pool}; // We want to see logs while testing.

/// Can we make a new file?
#[test]
fn create_blank() {
    // Make a blank file
    let _fs = get_filesystem();
    let root_block = Pool::root_directory(None).unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    assert_eq!(new_file.get_size(), 0); // Brand new files should be empty.
}

/// Can we make a new small file?
#[test]
fn write_small_file() {
    // Make a blank file
    let fs = get_filesystem();
    let root_block = Pool::root_directory(None).unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // Bytes to write
    let mut random: ThreadRng = rand::rng();
    let mut bytes: [u8; 512] = [0_u8; 512];

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_file.write(&bytes, seek_point, None).unwrap();

    // Make sure we actually wrote all the bytes
    assert_eq!(bytes_written, bytes.len() as u64);
    //  This should fit on one disk.
    assert_eq!(fs.pool.lock().expect("testing").header.highest_known_disk, 1);
}

/// Can we make a new small file?
#[test]
fn write_big_file() {
    // Make a blank file
    let fs = get_filesystem();
    let root_block = Pool::root_directory(None).unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // lol, how about 4 MB
    const FOUR_MEG: usize = 4 * 1024 * 1024;
    let mut random: ThreadRng = rand::rng();
    let mut bytes: Vec<u8> = Vec::with_capacity(FOUR_MEG);
    bytes.resize_with(FOUR_MEG, || 0);

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_file.write(&bytes, seek_point, None).unwrap();

    // Make sure we actually wrote all the bytes
    assert_eq!(bytes_written, bytes.len() as u64);
    // Make sure this actually used multiple disks
    assert!(fs.pool.lock().expect("testing").header.highest_known_disk > 1);
}

/// Make a lot of empty random files.
#[test]
fn make_lots_of_files() {
    // Make a blank file
    let _fs = get_filesystem();
    let mut current_filename_number: usize = 0;
    for _ in 0..1000 {
        let root_block = Pool::root_directory(None).unwrap();
        let new_name: String = format!("{current_filename_number}.txt");
        let _new_file = root_block.new_file(new_name, None).unwrap();
        // we wont write anything.
        current_filename_number += 1;
    }
}

/// Make a lot of filled random size files.
#[test]
fn make_lots_of_filled_files() {
    let _fs = get_filesystem();
    let mut current_filename_number: usize = 0;
    let mut random: ThreadRng = rand::rng();
    let mut total_bytes_written: u64 = 0;
    for _ in 0..1000 {
        let root_block = Pool::root_directory(None).unwrap();
        let new_name: String = format!("{current_filename_number}.txt");
        let new_file = root_block.new_file(new_name, None).unwrap();
        

        // Random size between 1 byte and 1 MB
        let file_size: usize = random.random_range(1..1024*1024);
        let mut bytes: Vec<u8> = Vec::with_capacity(file_size);
        bytes.resize_with(file_size, || 0);

        random.fill_bytes(&mut bytes);

        // Write to that file
        let bytes_written = new_file.write(&bytes, 0, None).unwrap();

        // Make sure we actually wrote all the bytes
        assert_eq!(bytes_written, bytes.len() as u64);
        // Keep track in case we need to debug
        total_bytes_written += bytes_written;
        current_filename_number += 1;
    }
}