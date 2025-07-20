// Files, direct to thee.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use log::debug;
use tempfile::{tempdir, TempDir};

use crate::{filesystem::filesystem_struct::{FilesystemOptions, FlusterFS}, pool::{disk::{drive_struct::FloppyDrive, generic::{generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::block::directory::directory_struct::DirectoryBlock}, pool_actions::pool_struct::Pool}};

use test_log::test; // We want to see logs while testing.

// Since these tests touch global state, they need to be forked, otherwise they will collide.

#[test]
fn add_directory() {
    // Use the filesystem starter to get everything in the right spots
    let _fs = get_filesystem();
    // Now try adding a directory to the pool
    let block = get_directory_block();
    let origin: DiskPointer = DiskPointer { disk: 1, block: 2 };
    block.make_directory("test".to_string(), origin).unwrap();
    // TODO: Add cleanup/shutdown method to the filesystem that wipes the globals.
}


// We need a filesystem to run directory tests on.
fn get_filesystem() -> FlusterFS {
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    FlusterFS::start(&fs_options)
}

// Get the directory block from the fresh file system
fn get_directory_block() -> DirectoryBlock {
    // This assumes you already started the filesystem

    // Now grab the first DirectoryBlock
    let block = match FloppyDrive::open(1).unwrap() {
        crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => {
            let raw = standard_disk.checked_read(2).unwrap();
            DirectoryBlock::from_block(&raw)
        },
        _ => panic!("Non standard disk."),
    };
    block
}

//
// Helper functions
//

// Temporary directories for virtual disks
fn get_new_temp_dir() -> TempDir {
    let mut dir = tempdir().unwrap();
    dir.disable_cleanup(true);
    debug!("Created a temp directory at {}, it will not be deleted on exit.", dir.path().to_string_lossy());
    dir
}
// TODO: This is getting dropped which removes the temporary disks, how do we keep it in scope?