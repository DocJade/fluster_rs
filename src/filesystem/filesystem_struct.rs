// This is where the fun begins

// Imports

use crate::pool::pool_actions::pool_struct::Pool;
use easy_fuser::templates::DefaultFuseHandler;
use std::{path::PathBuf, sync::{Arc, Mutex}};
// Structs, Enums, Flags

pub struct FlusterFS {
    pub(crate) inner: Box<DefaultFuseHandler>,
    pub(crate) pool: Arc<Mutex<Pool>>,
}

use lazy_static::lazy_static;

// Global varibles
// We need to access the path quite deep down into the disk functions, passing it all the way down there would be silly.
// Same with the virtual disk flag.
lazy_static! {
    pub(crate) static ref USE_VIRTUAL_DISKS: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub(crate) static ref FLOPPY_PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

/// Options availble at time of pool creation / filesystem load
pub struct FilesystemOptions {
    /// Use virtual disks in a temp folder instead of accessing the floppy drive.
    /// This option is used for testing.
    pub(super) use_virtual_disks: Option<PathBuf>,
    /// The location of the floppy drive block device
    pub(super) floppy_drive: PathBuf,
}
