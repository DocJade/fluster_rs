// This is where the fun begins

use std::{path::PathBuf, sync::Mutex};

use easy_fuser::templates::DefaultFuseHandler;

use crate::pool::pool_struct::Pool;

pub struct FlusterFS {
    pub(super) inner: Box<DefaultFuseHandler>,
    pub(super) pool_info: Pool
}

use lazy_static::lazy_static;

// Global varibles
// We need to access the path quite deep down into the disk functions, passing it all the way down there would be silly.
// Same with the virtual disk flag.
lazy_static! {
    pub(crate) static ref USE_VIRTUAL_DISKS:  Mutex<bool> = Mutex::new(false);
    pub(crate) static ref FLOPPY_PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

/// Options availble at time of pool creation / filesystem load
pub struct FilesystemOptions {
    /// Use virtual disks in a temp folder instead of accessing the floppy drive.
    /// This option is used for testing.
    pub(super) use_virtual_disks: bool,
    /// The location of the floppy drive block device
    pub(super) floppy_drive: PathBuf,
}
impl FilesystemOptions {
    pub fn new(use_virtual_disks: bool, floppy_drive: PathBuf) -> Self {
        // Set the globals
        // set the floppy disk path
        *FLOPPY_PATH.lock().expect("Fluster! Is single threaded.") = floppy_drive.clone();

        // Set the virtual disk flag
        *USE_VIRTUAL_DISKS.lock().expect("Fluster! Is single threaded.") = use_virtual_disks;
        Self {
            use_virtual_disks,
            floppy_drive,
        }
    }
}