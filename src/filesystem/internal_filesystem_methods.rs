// For stuff like initialization and options.

//
//
// ======
// Imports
// ======
//
//

use std::path::PathBuf;
use std::process::exit;

use log::debug;

use crate::pool::pool_actions::pool_struct::Pool;
use crate::filesystem::filesystem_struct::FilesystemOptions;
use crate::filesystem::filesystem_struct::FlusterFS;
use crate::filesystem::filesystem_struct::FLOPPY_PATH;
use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;


//
//
// ======
// Implementations
// ======
//
//


// Filesystem option setup. Does not start filesystem.
impl FilesystemOptions {
    /// Initializes options for the filesystem, also configures the virtual disks if needed.
    pub fn new(use_virtual_disks: Option<PathBuf>, floppy_drive: PathBuf) -> Self {
        debug!("Configuring file system options...");
        // Set the globals
        // set the floppy disk path
        debug!("Setting the floppy path...");
        debug!("Locking FLOPPY_PATH...");
        *FLOPPY_PATH
            .try_lock()
            .expect("Fluster! Is single threaded.") = floppy_drive.clone();
        debug!("Done.");

        // Set the virtual disk flag if needed
        if let Some(path) = use_virtual_disks.clone() {
            debug!("Setting up virtual disks...");
            // Sanity checks
            // Make sure this is a directory, and that the directory already exists
            if !path.is_dir() || !path.exists() {
                // Why must you do this
                println!("Virtual disk argument must be a valid path to a pre-existing directory.");
                exit(-1);
            }

            debug!("Locking USE_VIRTUAL_DISKS...");
            *USE_VIRTUAL_DISKS
                .try_lock()
                .expect("Fluster! Is single threaded.") = Some(path.to_path_buf());
            debug!("Done.");
        };

        debug!("Done configuring.");
        Self {
            use_virtual_disks,
            floppy_drive,
        }
    }
}

// Starting the filesystem.
impl FlusterFS {
    /// Create new filesystem handle, this will kick off the whole process of loading in information about the pool.
    /// Takes in options to configure the new pool.
    pub fn start(options: &FilesystemOptions) -> Self {
        debug!("Starting file system...");
        // Right now we dont use the options for anything, but they do initialize the globals we need, so we still need to pass it in.
        #[allow(dead_code)]
        #[allow(unused_variables)]
        let unused = options;
        let fs = FlusterFS { pool: Pool::load() };
        debug!("Done starting filesystem.");
        fs
    }
}