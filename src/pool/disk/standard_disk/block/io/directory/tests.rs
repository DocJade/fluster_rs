// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use log::debug;
use rand::{rngs::ThreadRng, seq::IndexedRandom, Rng};
use tempfile::{tempdir, TempDir};

use crate::{filesystem::filesystem_struct::{FilesystemOptions, FlusterFS}, pool::{disk::{drive_struct::FloppyDrive, generic::{generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::block::{directory::directory_struct::DirectoryBlock, io::directory::types::NamedItem}}, pool_actions::pool_struct::Pool}};

use test_log::test; // We want to see logs while testing.

// Since these tests touch global state, they need to be forked, otherwise they will collide.

#[test]
fn add_directory() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let block = Pool::root_directory(None).unwrap();
    let origin: DiskPointer = DiskPointer { disk: 1, block: 2 };
    block.make_directory("test".to_string(), None).unwrap();
    // We dont even check if its there, we just want to know if writing it failed.
}

#[test]
fn add_directory_and_list() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let block = Pool::root_directory(None).unwrap();
    let origin: DiskPointer = DiskPointer { disk: 1, block: 2 };
    block.make_directory("test".to_string(), None).unwrap();
    
    // try to find it again
    let new_block = Pool::root_directory(None).unwrap();
    assert!(new_block.contains_item(&NamedItem::Directory("test".to_string()), None).unwrap().is_some());
}

#[test]
fn nested_directory_hell() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    let mut random: ThreadRng = rand::rng();
    let mut name_number: usize = 0;
    
    // Create random directories at random places.
    for _ in 0..10000 {
        // Load in the root
        let mut where_are_we = Pool::root_directory(None).unwrap();
        // We will open random directories a few times, if they exist.
        loop {
            // List the current directory
            let square_holes = where_are_we.list(None).unwrap();
            // If there is no directories at this level, we're done.
            if square_holes.is_empty() {
                break
            }
            // Random chance to not go any deeper.
            if random.random_bool(0.5) {
                // not going any further.
                break
            }
            // Looks like we're entering a new directory.
            let destination = square_holes.choose(&mut random).expect("Already checked if it was empty.").name.clone();
            println!("{destination}");
            // Go forth!
            where_are_we = where_are_we.change_directory(destination, None).unwrap().unwrap();
            continue;
        }
        // Now that we've picked a directory, lets make a new one in here.
        // To make sure we dont end up with duplicate directory names, we just use a counter.
        where_are_we.make_directory(name_number.to_string(), None).unwrap();
        name_number += 1;
    }
}


// We need a filesystem to run directory tests on.
fn get_filesystem() -> FlusterFS {
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    FlusterFS::start(&fs_options)
    // We don't actually have to mount it for non-integration testing.
}

// Temporary directories for virtual disks
pub fn get_new_temp_dir() -> TempDir {
    let mut dir = tempdir().unwrap();
    dir.disable_cleanup(true);
    debug!("Created a temp directory at {}, it will not be deleted on exit.", dir.path().to_string_lossy());
    dir
}