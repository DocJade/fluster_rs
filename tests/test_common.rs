use std::path::PathBuf;

use easy_fuser::prelude::MountOption;
use fluster_fs::filesystem::filesystem_struct::{FilesystemOptions, FlusterFS};
use log::{debug, info};
use tempfile::{TempDir, tempdir};

//
// Helper functions
//

// Temporary directories for virtual disks
pub fn get_new_temp_dir() -> TempDir {
    info!("Getting a persistent temp dir for testing...");
    let mut dir = tempdir().unwrap();
    dir.disable_cleanup(true);
    debug!(
        "Created a temp directory at {}, it will not be deleted on exit.",
        dir.path().to_string_lossy()
    );
    dir
}

// Temporary directories for virtual disks
pub fn get_actually_temp_dir() -> TempDir {
    info!("Getting a non-persistent temp dir for testing...");
    tempdir().unwrap()
}

// Create a temporary filesystem, and returns the
pub fn start_filesystem() -> FlusterFS {
    info!("Starting temp test filesystem...");
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    FlusterFS::start(&fs_options)
}

pub fn unmount(mount_point: PathBuf) {
    info!("Unmounting filesystem....");
    let _ = std::process::Command::new("fusermount")
        .arg("-u")
        .arg(mount_point)
        .status();
}

pub fn test_mount_options() -> Vec<MountOption> {
    [
        MountOption::NoDev, // Disable dev devices
        MountOption::NoAtime, // No access times
        MountOption::NoSuid, // Ignore file/folder permissions (lol)
        MountOption::RW, // Read/Write
        MountOption::Exec, // Files are executable
        MountOption::Sync, // No async.
        MountOption::DirSync // No async
    ].to_vec()
}