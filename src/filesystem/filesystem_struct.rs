// This is where the fun begins

// Imports

use crate::pool::pool_actions::pool_struct::Pool;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};
// Structs, Enums, Flags

pub struct FlusterFS {
    #[allow(dead_code)] // it's lying.
    pub(crate) pool: Arc<Mutex<Pool>>,
}

use lazy_static::lazy_static;

// Global varibles
// We need to access the path quite deep down into the disk functions, passing it all the way down there would be silly.
// Same with the virtual disk flag.
lazy_static! {
    /// Use virtual disks instead of actually mounting the provided floppy drive path.
    pub(crate) static ref USE_VIRTUAL_DISKS: Mutex<Option<PathBuf>> = Mutex::new(None);
    /// The full path to the floppy drive.
    pub(crate) static ref FLOPPY_PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

// Backups cannot be disabled at runtime, so we use a once lock for them
/// Enable and disable backing up disks to /var/fluster
pub(crate) static WRITE_BACKUPS: OnceLock<bool> = OnceLock::new();
// TUI cannot be disabled mid run.
pub(crate) static USE_TUI: OnceLock<bool> = OnceLock::new();

/// Options availble at time of pool creation / filesystem load
pub struct FilesystemOptions {
    /// Use virtual disks in a temp folder instead of accessing the floppy drive.
    /// This option is used for testing.
    #[allow(dead_code)] // it's lying.
    pub(super) use_virtual_disks: Option<PathBuf>,
    /// The location of the floppy drive block device
    #[allow(dead_code)] // it's lying.
    pub(super) floppy_drive: PathBuf,
    /// Enable backing up disks to /var/fluster
    #[allow(dead_code)] // it's lying.
    pub(super) enable_backup: bool,
    /// Enable the TUI
    #[allow(dead_code)] // it's lying.
    pub(super) enable_tui: bool
}
