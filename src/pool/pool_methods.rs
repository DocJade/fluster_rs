// Interacting with the pool

// Imports

use crate::pool::disk::drive_struct::DiskBootstrap;
use crate::pool::disk::drive_struct::DiskType;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::drive_struct::FloppyDriveError;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;
use crate::pool::pool_struct::Pool;
use crate::pool::pool_struct::PoolStatistics;
use crate::pool::pool_struct::GLOBAL_POOL;
use log::debug;
use log::error;
use std::cell::RefCell;
use std::process::exit;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

// Implementations

impl Pool {
    /// Sync information about the pool to disk
    pub fn sync(&self) -> Result<(), ()> {
        sync(self)
    }
    /// Read in pool information from disk
    /// Returns a handle/pointer/whatever
    pub fn load() -> Arc<Mutex<Pool>> {
        load()
    }
    /// Brand new pools need to run some setup functions to get everything in a ready to use state.
    fn initalize(&mut self) -> Result<(), FloppyDriveError> {
        initalize_pool(self)
    }
}

impl PoolStatistics {
    fn new() -> Self {
        PoolStatistics {
            swaps: 0,
            data_bytes_read: 0,
            total_bytes_read: 0,
            data_bytes_written: 0,
            total_bytes_written: 0,
            cache_hit_rate: 0.0,
        }
    }
}

/// Sync information about the pool to disk
pub(super) fn sync(pool: &Pool) -> Result<(), ()> {
    todo!()
}

/// Read in pool information from disk.
/// Will prompt to make new pools if needed.
/// Returns a pointer thingy to to the global.
pub(super) fn load() -> Arc<Mutex<Pool>> {
    // Read in the header. If this fails, we cannot start the filesystem.
    let header = match PoolDiskHeader::read() {
        Ok(ok) => ok,
        Err(error) => {
            // We cannot start the pool without reading in the header!
            error!("Failed to acquire pool header! {error}");
            println!("Failed to load the pool.");
            println!("Reason: {error}");
            println!("Fluster will now exit.");
            exit(-1);
        }
    };

    let mut pool = Pool {
        header,
        statistics: PoolStatistics::new(),
    };

    // Wrap it for sharing.
    let shared_pool = Arc::new(Mutex::new(pool));
    
    // Set the global static. This will only work the first time.
    GLOBAL_POOL.set(shared_pool.clone()).expect("Pool already loaded");

    // All operations after this point use the global pool.

    let mut neighbors_pool =
        GLOBAL_POOL
            .get()
            .expect("Fluster is single threaded.")
            .lock()
            .expect("Fluster is single threaded.");

    // Check if this is a brand new pool
    if neighbors_pool.header.highest_known_disk == 0 {
        // This is a brand new pool, we need to initialize it.
        match neighbors_pool.initalize() {
            Ok(ok) => ok,
            Err(error) => {
                // Initializing the pool failed. This cannot continue.
                error!("Failed to initalize pool! {error}");
                println!("Failed to load the pool.");
                println!("Reason: {error}");
                println!("Fluster will now exit.");
                exit(-1);
            }
        };
    };

    drop(neighbors_pool);

    // All done
    return shared_pool;
}

/// Set up stuff for a brand new pool
fn initalize_pool(pool: &mut Pool) -> Result<(), FloppyDriveError> {
    debug!("Doing first time pool setup...");
    // Things a pool needs:
    // A second disk to start storing inodes on.
    // A root directory.
    
    // Lets get that second disk going
    // First we need to make a standard disk
    let mut standard_disk = add_disk::<StandardDisk>(pool)?;

    // Make sure that disk is disk 1, otherwise we are cooked.
    assert_eq!(standard_disk.number, 1);

    // Time for that root directory inode.
    // Since its the first inode we're adding, there should be enough space.
    // TODO:
    
    // Now we need to add that root directory inode.
    todo!()
}

/// Add a new disk of Type to the pool.
/// Takes the next available disk number.
/// Returns the newly created disk of type T.
fn add_disk<T: DiskBootstrap>(pool: &mut Pool) -> Result<T, FloppyDriveError> {
    debug!("Attempting to add new disk to the pool of type: {}", std::any::type_name::<T>());
    let next_open_disk = pool.header.highest_known_disk + 1;
    // First, we need a blank disk.
    // For virtual disk reasons, we still need to pass in the disk number that
    // we wish to create.
    let blank_disk = FloppyDrive::get_blank_disk(next_open_disk)?;
    
    // Now we need to create a disk to put in there from the supplied generic
    let bootstrapped = T::bootstrap(blank_disk.disk_file(), next_open_disk)?;

    // The disk has now bootstrapped itself, we are done here.
    pool.header.highest_known_disk = next_open_disk;
    Ok(bootstrapped)
}
