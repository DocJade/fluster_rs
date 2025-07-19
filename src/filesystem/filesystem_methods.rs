// I might blow a fuse.

// At this level of abstraction, we make calls to the Pool type. Nothing lower.

// Imports

use super::filesystem_struct::FLOPPY_PATH;
use super::filesystem_struct::FilesystemOptions;
use super::filesystem_struct::FlusterFS;
use super::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::pool_struct::Pool;
use easy_fuser::{FuseHandler, templates::DefaultFuseHandler};
use std::path::PathBuf;
use std::process::exit;

// Implementations

impl FlusterFS {
    /// Create new filesystem handle, this will kick off the whole process of loading in information about the pool.
    /// Takes in options to configure the new pool.
    pub fn start(options: &FilesystemOptions) -> Self {
        // Right now we dont use the options for anything, but they do initialize the globals we need, so we still need to pass it in.
        #[allow(dead_code)]
        #[allow(unused_variables)]
        let unused = options;
        FlusterFS {
            inner: Box::new(DefaultFuseHandler::new()),
            pool: Pool::load(),
        }
    }
}

impl FilesystemOptions {
    /// Initializes options for the filesystem, also configures the virtual disks if needed.
    pub fn new(use_virtual_disks: Option<PathBuf>, floppy_drive: PathBuf) -> Self {
        // Set the globals
        // set the floppy disk path
        *FLOPPY_PATH.lock().expect("Fluster! Is single threaded.") = floppy_drive.clone();

        // Set the virtual disk flag if needed
        if let Some(ref path) = use_virtual_disks {
            // Sanity checks
            // Make sure this is a directory, and that the directory already exists
            if !path.is_dir() || !path.exists() {
                // Why must you do this
                println!("Virtual disk argument must be a valid path to a pre-existing directory.");
                exit(-1);
            }

            *USE_VIRTUAL_DISKS
                .lock()
                .expect("Fluster! Is single threaded.") = Some(path.to_path_buf());
        };

        Self {
            use_virtual_disks,
            floppy_drive,
        }
    }
}

//
// easy_fuser methods.
//

// We are using PathBufs as the unique identifier for paths instead of inode numbers, because inode numbers are scary.
impl FuseHandler<PathBuf> for FlusterFS {
    /// This does... Something, im not sure what, but we need it.
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        self.inner.as_ref()
    }
}
