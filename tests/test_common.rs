use std::{ffi::OsStr, path::PathBuf};

use fluster_fs::filesystem::filesystem_struct::{FilesystemOptions, FlusterFS};
use fuse_mt::FuseMT;
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

// Create a temporary filesystem, and returns the mt thing used to do the mount.
pub fn start_filesystem() -> FuseMT<FlusterFS> {
    info!("Starting temp test filesystem...");
    let temp_dir = get_new_temp_dir();
    let floppy_drive: PathBuf = PathBuf::new(); // This is never read since we are using temporary disks.
    let fs_options = FilesystemOptions::new(Some(temp_dir.path().to_path_buf()), floppy_drive);
    let started = FlusterFS::start(&fs_options);
    // MT thing that is actually used for mounting.
    fuse_mt::FuseMT::new(started, 1)
}

pub fn unmount(mount_point: PathBuf) {
    info!("Unmounting filesystem....");
    let _ = std::process::Command::new("fusermount")
        .arg("-u")
        .arg(mount_point)
        .status();
}

pub fn test_mount_options() -> Vec<&'static OsStr> {
    [
        OsStr::new("-o"), // Options flag
        OsStr::new("nodev"), // Disable dev devices
        OsStr::new("noatime"), // No access times
        OsStr::new("nosuid"), // Ignore file/folder permissions (lol)
        OsStr::new("rw"), // Read/Write
        OsStr::new("exec"), // Files are executable
        OsStr::new("sync"), // No async.
        OsStr::new("dirsync"), // No async
    ].to_vec()
}