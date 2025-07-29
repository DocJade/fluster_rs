// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use log::debug;
use rand::{Rng, rngs::ThreadRng, seq::IndexedRandom};
use tempfile::{TempDir, tempdir};

use crate::{
    filesystem::filesystem_struct::{FilesystemOptions, FlusterFS},
    pool::{
        disk::{
            standard_disk::block::io::directory::types::NamedItem,
        },
        pool_actions::pool_struct::Pool,
    },
};

use test_log::test; // We want to see logs while testing.

// Since these tests touch global state, they need to be forked, otherwise they will collide.

#[test]
fn add_directory() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let block = Pool::root_directory().unwrap();
    block.make_directory("test".to_string(), None).unwrap();
    // We dont even check if its there, we just want to know if writing it failed.
}

#[test]
fn add_directory_and_list() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let block = Pool::root_directory().unwrap();
    block.make_directory("test".to_string(), None).unwrap();

    // try to find it again
    let new_block = Pool::root_directory().unwrap();
    assert!(
        new_block
            .find_item(&NamedItem::Directory("test".to_string()), None)
            .unwrap()
            .is_some()
    );
}

#[test]
#[ignore = "Takes way too long to run with debug logging, must run standalone with a level of INFO if you ever want to see your family again."]
fn nested_directory_hell() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    let mut random: ThreadRng = rand::rng();
    let mut name_number: usize = 0;

    // Create random directories at random places.
    for _ in 0..10_000 {
        // Load in the root
        let mut where_are_we = Pool::root_directory().unwrap();
        // We will open random directories a few times, if they exist.
        loop {
            // List the current directory
            let square_holes = where_are_we.list(None).unwrap();
            // If there is no directories at this level, we're done.
            if square_holes.is_empty() {
                break;
            }
            // Random chance to not go any deeper.
            if random.random_bool(0.2) {
                // Incentivize deep nesting.
                // not going any further.
                break;
            }
            // Looks like we're entering a new directory.
            let destination = square_holes
                .choose(&mut random)
                .expect("Already checked if it was empty.")
                .name
                .clone();
            // Go forth!
            where_are_we = where_are_we
                .change_directory(destination, None)
                .unwrap()
                .unwrap();
            continue;
        }
        // Now that we've picked a directory, lets make a new one in here.
        // To make sure we dont end up with duplicate directory names, we just use a counter.
        where_are_we
            .make_directory(name_number.to_string(), None)
            .unwrap();
        name_number += 1;
    }
}

#[test]
/// Ensure that directories eventually start reporting other disks besides disk 1 by
/// writing way too many directories.
fn directories_switch_disks() -> Result<(), ()> {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    for i in 0..3000 {
        // There's only 2880 blocks on the first disk, assuming no overhead.
        let root_dir = Pool::root_directory().unwrap();
        root_dir.make_directory(i.to_string(), None).unwrap();
    }
    // Now make sure we actually have directories that claim to live on another disk
    let root_dir_done = Pool::root_directory().unwrap();
    for dir in root_dir_done.list(None).unwrap() {
        if dir.location.disk.unwrap() != 1 {
            // Made it to another disk.
            debug!(
                "Made it onto another disk! Disk: {}",
                dir.location.disk.unwrap()
            );
            return Ok(());
        }
    }
    // They were all disk 1!
    panic!("All directories are on disk 1!");
}

// We need a filesystem to run directory tests on.
pub fn get_filesystem() -> FlusterFS {
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
    debug!(
        "Created a temp directory at {}, it will not be deleted on exit.",
        dir.path().to_string_lossy()
    );
    dir
}
