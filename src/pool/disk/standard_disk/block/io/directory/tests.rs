// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use log::debug;
use rand::{rngs::ThreadRng, seq::{IndexedRandom, SliceRandom}, Rng};
use tempfile::{TempDir, tempdir};

use crate::{
    filesystem::filesystem_struct::{FilesystemOptions, FlusterFS},
    pool::{
        disk::standard_disk::block::{directory::directory_struct::DirectoryItem, io::directory::types::NamedItem},
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
    let mut block = Pool::get_root_directory().unwrap();
    let _ = block.make_directory("test".to_string()).unwrap();
    // We dont even check if its there, we just want to know if writing it failed.
}

#[test]
// Make sure creating a file only makes one entry.
fn creating_only_makes_one_directory() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let mut block = Pool::get_root_directory().unwrap();
    let result = block.make_directory("test".to_string()).unwrap();

    let listed = block.list().unwrap();

    // There should only be one directory.
    assert_eq!(listed.len(), 1);
    
    // The returned item should be the same
    assert_eq!(listed[0], result);
}

#[test]
fn add_and_delete_directory() {
    let _fs = get_filesystem();
    let mut block = Pool::get_root_directory().unwrap();
    let _ = block.make_directory("test".to_string()).unwrap();
    
    // Now delete that directory

    // Extract it
    let test_dir_item = block.find_and_extract_item(&NamedItem::Directory("test".to_string())).unwrap().unwrap();

    // Call delete on it
    test_dir_item.get_directory_block().unwrap().delete_self(test_dir_item).unwrap();

    // Directory should now be empty
    assert!(block.list().unwrap().is_empty());
}

#[test]
// Make sure directories shrink when items are removed.
fn deletion_shrinks() {
    let _fs = get_filesystem();
    let mut block = Pool::get_root_directory().unwrap();
    
    // make a bunch of directories in here with large names to quickly expand the block
    for i in 0..200 {
        let _ = block.make_directory(format!("test_this_is_a_long_name_to_use_more_space_lol_{i}")).unwrap();
    }
    
    // Remove all of them.
    for i in 0..200 {
        let delete_me = block.find_and_extract_item(&NamedItem::Directory(format!("test_this_is_a_long_name_to_use_more_space_lol_{i}"))).unwrap().unwrap();
        delete_me.get_directory_block().unwrap().delete_self(delete_me).unwrap()
    }
    
    // Now make sure the empty block is only 1 block large.
    assert!(block.next_block.no_destination());

    // Should also contain nothing
    assert!(block.list().unwrap().is_empty());
}

#[test]
// Try renaming some items
fn rename_items() {
    let _fs = get_filesystem();
    let mut block = Pool::get_root_directory().unwrap();
    
    // A lot of directories
    let mut directories: Vec<DirectoryItem> = Vec::new();
    for i in 0..100 {
        directories.push(block.make_directory(format!("dir_{i}")).unwrap());
    }

    // A lot of files
    let mut files: Vec<DirectoryItem> = Vec::new();
    for i in 0..100 {
        files.push(block.new_file(format!("file_{i}.txt")).unwrap());
    }

    // Shuffle for fun
    let mut all_items: Vec<DirectoryItem> = Vec::new();
    all_items.extend(directories);
    all_items.extend(files);

    let mut random: ThreadRng = rand::rng();

    all_items.shuffle(&mut random);

    // How many do we have
    let number_made: usize = all_items.len();



    // Go rename all of them
    for item in all_items {
        // need a new name... hmmmmm
        let new_name: String = format!("new_{}", item.name);
        let renamed = block.try_rename_item(&item.into(), new_name).unwrap();
        assert!(renamed)
    }

    // Make sure the directory still contains the correct number of items. (ie we didn't duplicate anything.)
    let list = block.list().unwrap();
    assert_eq!(number_made, list.len());
}

#[test]
fn add_directory_and_list() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let mut block = Pool::get_root_directory().unwrap();
    let _ = block.make_directory("test".to_string(),).unwrap();

    // try to find it again
    let new_block = Pool::get_root_directory().unwrap();
    assert!(
        new_block
            .find_item(&NamedItem::Directory("test".to_string()),)
            .unwrap()
            .is_some()
    );
}

#[test]
fn nested_directory_hell() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    let mut random: ThreadRng = rand::rng();
    let mut name_number: usize = 0;

    // Create random directories at random places.
    for _ in 0..10_000 {
        // Load in the root
        let mut where_are_we = Pool::get_root_directory().unwrap();
        // We will open random directories a few times, if they exist.
        loop {
            // List the current directory
            let square_holes = where_are_we.list().unwrap();
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
            // Random chance to go back to the root for more chaos.
            if random.random_bool(0.05) {
                // Back to root!
                where_are_we = Pool::get_root_directory().unwrap();
                continue;
            }
            // Looks like we're entering a new directory.
            let destination = square_holes
                .choose(&mut random)
                .expect("Already checked if it was empty.")
                .name
                .clone();
            // Go forth!
            where_are_we = where_are_we
                .change_directory(destination)
                .unwrap()
                .unwrap();
            continue;
        }
        // Now that we've picked a directory, lets make a new one in here.
        // To make sure we dont end up with duplicate directory names, we just use a counter.
        let _ = where_are_we
            .make_directory(name_number.to_string())
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
        let mut root_dir = Pool::get_root_directory().unwrap();
        let _ = root_dir.make_directory(i.to_string()).unwrap();
    }
    // Now make sure we actually have directories that claim to live on another disk
    let root_dir_done = Pool::get_root_directory().unwrap();
    for dir in root_dir_done.list().unwrap() {
        if dir.location.pointer.disk != 1 {
            // Made it to another disk.
            debug!(
                "Made it onto another disk! Disk: {}",
                dir.location.pointer.disk
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
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive, Some(false), false);
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
