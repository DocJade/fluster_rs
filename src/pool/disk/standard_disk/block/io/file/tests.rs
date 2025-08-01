// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]
use rand::{rngs::ThreadRng, Rng, RngCore};
use test_log::test;

use crate::pool::{disk::{generic::io::cache::cache_io::CachedBlockIO, standard_disk::block::{directory::directory_struct::DirectoryItem, io::directory::{tests::get_filesystem, types::NamedItem}}}, pool_actions::pool_struct::Pool}; // We want to see logs while testing.

/// Can we make a new file?
#[test]
fn create_blank() {
    // Make a blank file
    let _fs = get_filesystem();
    let root_block = Pool::root_directory().unwrap();
    let new_item = root_block.new_file("test123.txt".to_string(), None).unwrap();

    let new_file = new_item.get_inode().unwrap().extract_file().unwrap();

    assert_eq!(new_file.get_size(), 0); // Brand new files should be empty.
}

/// Can we make a new small file?
#[test]
fn write_small_file() {
    // Make a blank file
    let fs = get_filesystem();
    let root_block = Pool::root_directory().unwrap();
    let new_item = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // Bytes to write
    let mut random: ThreadRng = rand::rng();
    let mut bytes: [u8; 512] = [0_u8; 512];

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_item.write_file(&bytes, seek_point, None).unwrap();

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
    let root_block = Pool::root_directory().unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // lol, how about 4 MB
    // This is 4x what fluster will normally handle
    const FOUR_MEG: usize = 4 * 1024 * 1024;
    let mut random: ThreadRng = rand::rng();
    let mut bytes: Vec<u8> = Vec::with_capacity(FOUR_MEG);
    bytes.resize_with(FOUR_MEG, || 0);

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_file.write_file(&bytes, seek_point, None).unwrap();

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
        let root_block = Pool::root_directory().unwrap();
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
        let root_block = Pool::root_directory().unwrap();
        let new_name: String = format!("{current_filename_number}.txt");
        let new_file = root_block.new_file(new_name, None).unwrap();
        

        // Random size between 1 byte and 1 MB
        let file_size: usize = random.random_range(1..1024*1024);
        let mut bytes: Vec<u8> = Vec::with_capacity(file_size);
        bytes.resize_with(file_size, || 0);

        random.fill_bytes(&mut bytes);

        // Write to that file
        let bytes_written = new_file.write_file(&bytes, 0, None).unwrap();

        // Make sure we actually wrote all the bytes
        assert_eq!(bytes_written, bytes.len() as u64);
        // Keep track in case we need to debug
        total_bytes_written += bytes_written;
        current_filename_number += 1;
    }
}


/// Can we properly read and write a test file?
#[test]
fn write_and_read_small() {
    // Make a blank file
    let _fs = get_filesystem();
    let root_block = Pool::root_directory().unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // Bytes to write
    let mut random: ThreadRng = rand::rng();
    let mut bytes: [u8; 512] = [0_u8; 512];

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_file.write_file(&bytes, seek_point, None).unwrap();

    // Make sure we actually wrote all the bytes
    assert_eq!(bytes_written, bytes.len() as u64);

    // Read back in that file
    // We will find the file by its file name, to ensure disk access works correctly.

    let root_block = Pool::root_directory().unwrap();
    // Go fetch
    let named: NamedItem = NamedItem::File("test123.txt".to_string());
    let read_me = root_block.find_item(&named, None).unwrap().expect("We just made it");

    // Read the contained data
    let read_data = read_me.read_file(0, 512, None).unwrap();

    // Does it match?
    assert_eq!(read_data.len(), bytes.len());
    assert_eq!(read_data, bytes);
}

/// Can we properly read and write a test file?
#[test]
fn write_and_read_large() {
    // Make a blank file
    let _fs = get_filesystem();
    let root_block = Pool::root_directory().unwrap();
    let new_file = root_block.new_file("test123.txt".to_string(), None).unwrap();

    // Bytes to write
    const FOUR_MEG: usize = 4 * 1024 * 1024;
    let mut random: ThreadRng = rand::rng();
    let mut bytes: Vec<u8> = Vec::with_capacity(FOUR_MEG);
    bytes.resize_with(FOUR_MEG, || 0);

    random.fill_bytes(&mut bytes);

    // Write to that file
    // We will write from the start.
    let seek_point: u64 = 0;
    let bytes_written = new_file.write_file(&bytes, seek_point, None).unwrap();

    // Make sure we actually wrote all the bytes
    assert_eq!(bytes_written, bytes.len() as u64);

    // Read back in that file
    // We will find the file by its file name, to ensure disk access works correctly.

    let root_block = Pool::root_directory().unwrap();
    // Go fetch
    let named: NamedItem = NamedItem::File("test123.txt".to_string());
    let read_me = root_block.find_item(&named, None).unwrap().expect("We just made it");

    // Read the contained data
    let read_data = read_me.read_file(0, FOUR_MEG.try_into().unwrap(), None).unwrap();

    // Does it match?
    assert_eq!(read_data.len(), bytes.len());
    assert_eq!(read_data, bytes);
}


/// Read and write a lot of random files
#[test]
fn read_and_write_random_files() {
    let fs = get_filesystem();
    let mut current_filename_number: usize = 0;
    let mut random: ThreadRng = rand::rng();
    let mut total_bytes_written: u64 = 0;
    let mut random_files: Vec<Vec<u8>> = Vec::new();
    const TEST_LENGTH: usize = 1000;
    const MAX_FILE_SIZE: usize = 1024 * 1024; // Currently one meg
    for _ in 0..TEST_LENGTH {
        let root_block = Pool::root_directory().unwrap();
        let new_name: String = format!("{current_filename_number}.txt");
        let new_file = root_block.new_file(new_name, None).unwrap();
        
        
        // Random size between 1 byte and 1 MB
        let file_size: usize = random.random_range(1..MAX_FILE_SIZE);
        let mut bytes: Vec<u8> = Vec::with_capacity(file_size);
        bytes.resize_with(file_size, || 0);
        
        random.fill_bytes(&mut bytes);
        
        // Keep track of this file
        random_files.push(bytes.clone());
        
        // Write to that file
        let bytes_written = new_file.write_file(&bytes, 0, None).unwrap();
        
        // Make sure we actually wrote all the bytes
        assert_eq!(bytes_written, bytes.len() as u64);
        // Keep track in case we need to debug
        total_bytes_written += bytes_written;
        current_filename_number += 1;
    }
    let stats = fs.pool.lock().unwrap();
    let cache_hit_rate = CachedBlockIO::get_hit_rate();
    drop(stats);
    let _ = cache_hit_rate;

    // Now we need to read all of the files back out
    let mut current_file: usize = 0;
    let root_block = Pool::root_directory().unwrap();
    for _ in 0..TEST_LENGTH {
        let named: NamedItem = NamedItem::File(format!("{current_file}.txt"));
        let found: DirectoryItem = root_block.find_item(&named, None).unwrap().unwrap();

        // read it
        let file_size: u64 = found.get_inode().unwrap().extract_file().unwrap().get_size();
        let read = found.read_file(0, file_size.try_into().unwrap(), None).unwrap();

        // Compare
        assert_eq!(read, random_files[current_file]);
        
        // Next
        current_file += 1;
    }
    let stats = fs.pool.lock().unwrap();
    let cache_hit_rate = CachedBlockIO::get_hit_rate();
    panic!("test");
}