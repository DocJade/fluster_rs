// Inode tests.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use log::debug;
use tempfile::{TempDir, tempdir};

use crate::{
    filesystem::filesystem_struct::{FilesystemOptions, FlusterFS},
    pool::{
        disk::standard_disk::block::inode::inode_struct::Inode, pool_actions::pool_struct::Pool,
    },
};

use test_log::test; // We want to see logs while testing.

// In the inode test file, we already create some functions that we can reuse in here.

// Since these tests touch global state, they need to be isolated... somehow

#[test]
fn add_inode() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let _ = Pool::add_inode(Inode::get_random()).unwrap();
}

#[test]
fn add_many_inode() {
    let _fs = get_filesystem();
    for _ in 0..1000 {
        let _ = Pool::add_inode(Inode::get_random()).unwrap();
    }
}

// We need a filesystem to run directory tests on.
fn get_filesystem() -> FlusterFS {
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive, Some(false));
    FlusterFS::start(&fs_options)
}

//
// Helper functions
//

// Temporary directories for virtual disks
fn get_new_temp_dir() -> TempDir {
    let mut dir = tempdir().unwrap();
    dir.disable_cleanup(true);
    debug!(
        "Created a temp directory at {}, it will not be deleted on exit.",
        dir.path().to_string_lossy()
    );
    dir
}
