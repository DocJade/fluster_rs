// Interacting with the pool

// Imports

use super::pool_struct::GLOBAL_POOL;
use super::pool_struct::Pool;
use super::pool_struct::PoolStatistics;
use crate::pool::disk::drive_struct::DiskBootstrap;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::drive_struct::FloppyDriveError;
use crate::pool::disk::drive_struct::JustDiskType;
use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::generic::io::cache::cache_io::CachedBlockIO;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryBlock;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryFlags;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItem;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeLocation;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;
use log::debug;
use log::error;
use std::process::exit;
use std::sync::Arc;
use std::sync::Mutex;

// Implementations

impl Pool {
    /// Flush all info about the pool to the pool disk.
    pub fn flush() -> Result<(), FloppyDriveError> {
        flush_pool()
    }
    /// Read in pool information from disk
    /// Returns a handle/pointer/whatever
    pub fn load() -> Arc<Mutex<Pool>> {
        load()
    }
    /// Create a new disk of type and add it to the pool
    /// Returns that new disk.
    pub fn new_disk<T: DiskBootstrap>() -> Result<T, FloppyDriveError> {
        add_disk::<T>()
    }
    /// Brand new pools need to run some setup functions to get everything in a ready to use state.
    fn initalize() -> Result<(), FloppyDriveError> {
        initalize_pool()
    }
    /// Get the root inode block
    ///
    /// May swap disks, but you should be working with enough abstractions to not care.
    pub fn get_root_directory() -> Result<DirectoryBlock, FloppyDriveError> {
        pool_get_root_directory()
    }
    /// Get a DirectoryItem that has details about the root directory.
    pub fn get_root_directory_item() -> DirectoryItem {
        pool_get_root_directory_item()
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
        }
    }
}

/// Sync information about the pool to disk
pub(super) fn flush_pool() -> Result<(), FloppyDriveError> {
    debug!("Flushing pool info to disk...");
    
    // Grab the pool
    debug!("Locking GLOBAL_POOL...");
    let pool_header:PoolDiskHeader  = GLOBAL_POOL
        .get()
        .expect("single threaded")
        .try_lock()
        .expect("single threaded")
        .header;

    // Now write that back to disk.
    pool_header.write()?;
    debug!("Pool flushed.");
    Ok(())
}

/// Read in pool information from disk.
/// Will prompt to make new pools if needed.
/// Returns a pointer thingy to to the global.
pub(super) fn load() -> Arc<Mutex<Pool>> {
    debug!("Loading in pool information...");
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

    let pool = Pool {
        header,
        statistics: PoolStatistics::new(),
    };

    // Wrap it for sharing.
    let shared_pool = Arc::new(Mutex::new(pool));

    // Set the global static. This will only work the first time.
    GLOBAL_POOL
        .set(shared_pool.clone())
        .expect("Pool already loaded");

    // All operations after this point use the global pool.

    debug!("Locking GLOBAL_POOL...");
    let highest_known: u16 = GLOBAL_POOL
        .get()
        .expect("single threaded")
        .try_lock()
        .expect("single threaded")
        .header
        .highest_known_disk;

    // Check if this is a brand new pool
    if highest_known == 0 {
        // This is a brand new pool, we need to initialize it.
        match Pool::initalize() {
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

    // All done
    shared_pool
}

/// Set up stuff for a brand new pool
fn initalize_pool() -> Result<(), FloppyDriveError> {
    debug!("Doing first time pool setup...");
    // Things a pool needs:
    // A second disk to start storing inodes on.
    // A root directory.

    // Lets get that second disk going
    // First we need to make a standard disk
    debug!("Creating the standard disk (disk 1)...");
    let standard_disk = add_disk::<StandardDisk>()?;

    // Make sure that disk is disk 1, otherwise we are cooked.
    assert_eq!(standard_disk.number, 1);

    // The root directory is set up on the disk side, so we're done.
    debug!("Finished first time pool setup.");
    Ok(())
}

/// Add a new disk of Type to the pool.
/// Takes the next available disk number.
/// Returns the newly created disk of type T.
fn add_disk<T: DiskBootstrap>() -> Result<T, FloppyDriveError> {
    debug!(
        "Attempting to add new disk to the pool of type: {}",
        std::any::type_name::<T>()
    );
    debug!("Locking GLOBAL_POOL...");
    let highest_known: u16 = GLOBAL_POOL
        .get()
        .expect("single threaded")
        .try_lock()
        .expect("single threaded")
        .header
        .highest_known_disk;
    let next_open_disk = highest_known + 1;

    // First, we need a blank disk in the drive.
    // For virtual disk reasons, we still need to pass in the disk number that
    // we wish to create.
    debug!("Getting a new blank disk...");
    let blank_disk = FloppyDrive::get_blank_disk(next_open_disk)?;

    // Now we need to create a disk to put in there from the supplied generic
    debug!("Bootstrapping the new disk...");
    let bootstrapped = T::bootstrap(blank_disk.disk_file(), next_open_disk)?;

    // The disk has now bootstrapped itself, we are done here.
    debug!("Locking GLOBAL_POOL...");
    GLOBAL_POOL
        .get()
        .expect("single threaded")
        .try_lock()
        .expect("single threaded")
        .header
        .highest_known_disk += 1;

    debug!("Done adding new disk.");
    Ok(bootstrapped)
}

/// Grabs the root inode block
fn pool_get_root_directory() -> Result<DirectoryBlock, FloppyDriveError> {
    // Root directory should always be at disk 1 block 2. We just assume that to be the case.
    // Why do we have a root inode that points to the root directory when its always in a static location?
    // Beats me, I forgot why I did that.

    let root_pointer: DiskPointer = DiskPointer {
        disk: 1,
        block: 2,
    };

    // Get the root directory block
    let block_reader: RawBlock = CachedBlockIO::read_block(root_pointer, JustDiskType::Standard)?;
    let block = DirectoryBlock::from_block(&block_reader);

    Ok(block)
}

/// Grabs the root inode location, duh
fn pool_get_root_inode_location() -> InodeLocation {
    InodeLocation {
        disk: Some(1),
        block: 1,
        offset: 0,
    }
}

/// Constructs a directory item with info about the root.
fn pool_get_root_directory_item() -> DirectoryItem {
    // The name of the root directory entry is always the delimiter.
    static DELIMITER: char = std::path::MAIN_SEPARATOR;
    DirectoryItem {
        flags: DirectoryFlags::IsDirectory,
        name_length: 0,
        name: DELIMITER.into(),
        location: pool_get_root_inode_location(),
    }
}