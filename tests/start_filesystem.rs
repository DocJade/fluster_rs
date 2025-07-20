// TDD? Do you mean Transport Tycoon Deluxe?
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use fluster_fs::filesystem::filesystem_struct::FilesystemOptions;
use fluster_fs::filesystem::filesystem_struct::FlusterFS;
use tempfile::{TempDir, tempdir};

use test_log::test; // We want to see logs while testing.

#[test]
// Try starting up the filesystem
fn initialize_filesystem() {
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    let _fs: FlusterFS = FlusterFS::start(&fs_options);
}

//
// Helper functions
//

// Temporary directories for virtual disks
fn get_new_temp_dir() -> TempDir {
    tempdir().unwrap()
}
