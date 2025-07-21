use std::path::PathBuf;

use fluster_fs::filesystem::filesystem_struct::{FilesystemOptions, FlusterFS};
use log::debug;
use tempfile::{tempdir, TempDir};

//
// Helper functions
//

// Temporary directories for virtual disks
pub fn get_new_temp_dir() -> TempDir {
    let mut dir = tempdir().unwrap();
    dir.disable_cleanup(true);
    debug!("Created a temp directory at {}, it will not be deleted on exit.", dir.path().to_string_lossy());
    dir
}

// Temporary directories for virtual disks
pub fn get_actually_temp_dir() -> TempDir {
    tempdir().unwrap()
}

// Create a temporary filesystem, and returns the 
pub fn start_filesystem() -> FlusterFS {
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    FlusterFS::start(&fs_options)
}

pub fn unmount(mount_point: PathBuf) {
    let _ = std::process::Command::new("fusermount")
        .arg("-u")
        .arg(mount_point)
        .status();
}