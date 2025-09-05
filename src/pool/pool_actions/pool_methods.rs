// Interacting with the pool

// Imports

use super::pool_struct::GLOBAL_POOL;
use super::pool_struct::Pool;
use crate::error_types::drive::DriveError;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::drive_struct::DiskBootstrap;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::generic::io::cache::cache_io::CachedBlockIO;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryBlock;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItemFlags;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItem;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeLocation;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;
use crate::tui::notify::NotifyTui;
use crate::tui::tasks::TaskType;
use log::debug;
use log::error;
use std::sync::Arc;
use std::sync::Mutex;

// Implementations

impl Pool {
    /// Flush all info about the pool to the pool disk.
    pub fn flush() -> Result<(), DriveError> {
        flush_pool()
    }
    /// Read in pool information from disk
    /// Returns a handle/pointer/whatever
    pub fn load() -> Arc<Mutex<Pool>> {
        load()
    }
    /// Create a new disk of type and add it to the pool
    /// Returns that new disk.
    pub fn new_disk<T: DiskBootstrap>() -> Result<T, DriveError> {
        add_disk::<T>()
    }
    /// Brand new pools need to run some setup functions to get everything in a ready to use state.
    fn initalize() -> Result<(), DriveError> {
        initalize_pool()
    }
    /// Get the root inode block
    ///
    /// May swap disks, but you should be working with enough abstractions to not care.
    pub fn get_root_directory() -> Result<DirectoryBlock, DriveError> {
        pool_get_root_directory()
    }
    /// Get a DirectoryItem that has details about the root directory.
    pub fn get_root_directory_item() -> DirectoryItem {
        pool_get_root_directory_item()
    }
}

/// Sync information about the pool to disk
pub(super) fn flush_pool() -> Result<(), DriveError> {
    debug!("Flushing pool info to disk...");
    
    // Grab the pool
    let global_pool = GLOBAL_POOL
        .get()
        .expect("The pool has to exist, otherwise we couldn't shut it down.");

    // Now, since we're flushing info, if we're shutting down after a panic, we need to be able to
    // flush the pool even if it got poisoned.
    global_pool.clear_poison();

    let pool_header:PoolDiskHeader = 
        global_pool.try_lock()
        .expect("Single threaded, already cleared poison.")
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

    // We try at most 10 times.
    let mut header: Option<PoolDiskHeader> = None;
    for _ in 0..10 {
        match PoolDiskHeader::read() {
            Ok(ok) =>{
                header = Some(ok);
                break
            },
            Err(error) => {
                // If no disk was inserted, yell at the user and try again
                if error == DriveError::DriveEmpty {
                    // Dumbass.
                    println!("Yo. The drive is empty. Actually put in the disk.");
                    continue;
                }
            }
        };
    };

    // Did we get it?
    let header: PoolDiskHeader = if let Some(read) = header {
        // All good.
        read
    } else {
        // Failed to load in the disk header.
        error!("Failed to acquire pool header after 10 tries! Giving up!");
        error!("Fluster has failed to load the pool header.");
        error!("Fluster will now exit.");
        panic!("Failed to get pool header!");
    };
    

    let pool = Pool {
        header,
    };

    // Wrap it for sharing.
    let shared_pool = Arc::new(Mutex::new(pool));

    // Set the global static. This will only work the first time.
    // Since this is only called on fluster startup, this should only ever be called once.
    // If we somehow hit here again, we'll just exit
    
    if GLOBAL_POOL.set(shared_pool.clone()).is_err() {
        // wow!
        panic!("Somehow we've loaded the pool twice!");
    }
    
    // All operations after this point use the global pool.
    
    // We're the only place that could possibly have access to the pool right now.
    // So if this lock fails, cooked.
    
    let highest_known: u16 = if let Some(global_pool) = GLOBAL_POOL.get() {
        if let Ok(innards) = global_pool.try_lock() {
            innards.header.highest_known_disk
        } else {
            panic!("Locking the global pool immediately after creation failed!");
        }
    } else {
        // ??????????
        panic!("The global pool does not exist, even though we JUST made it?");
    };

    // Check if this is a brand new pool
    if highest_known == 0 {
        // This is a brand new pool, we need to initialize it.
        match Pool::initalize() {
            Ok(ok) => ok,
            Err(error) => {
                // Initializing the pool failed. This cannot continue.
                error!("Failed to load the pool.");
                error!("Reason: {error}");
                error!("Fluster will now exit.");
                panic!("Failed to initalize pool! {error}");
            }
        };
    };

    // All done
    shared_pool
}

/// Set up stuff for a brand new pool
fn initalize_pool() -> Result<(), DriveError> {
    debug!("Doing first time pool setup...");
    // Things a pool needs:
    // A second disk to start storing inodes on.
    // A root directory.

    // Lets get that second disk going
    // First we need to make a standard disk
    debug!("Creating the standard disk (disk 1)...");
    let _ = add_disk::<StandardDisk>()?;

    // The root directory is set up on the disk side, so we're done.
    debug!("Finished first time pool setup.");
    Ok(())
}

/// Add a new disk of Type to the pool.
/// Takes the next available disk number.
/// Returns the newly created disk of type T.
fn add_disk<T: DiskBootstrap>() -> Result<T, DriveError> {
    let handle = NotifyTui::start_task(TaskType::CreateNewDisk, 2);
    debug!(
        "Attempting to add new disk to the pool of type: {}",
        std::any::type_name::<T>()
    );


    let highest_known: u16 = GLOBAL_POOL
        .get()
        .expect("Pool must exist at to add disks to it.")
        .try_lock()
        .expect("Cannot add disks to poisoned pool. Also single threaded, so should not block.")
        .header
        .highest_known_disk;
    let next_open_disk = highest_known + 1;

    // First, we need a blank disk in the drive.
    // For virtual disk reasons, we still need to pass in the disk number that
    // we wish to create.
    debug!("Getting a new blank disk...");
    // We loop until there is a disk in the drive, just in case.
    // try 10 times
    let blank_disk: BlankDisk;
    let mut tries: u8 = 0;
    loop {
        if let Ok(disk) = FloppyDrive::get_blank_disk(next_open_disk){
            blank_disk = disk;
            break
        };
        // We need that blank mf.
        if tries == 10 {
            // shiet
            error!("Couldnt get a blank disk! cooked!");
            return Err(DriveError::DriveEmpty); // I mean, good enough.
        }
        tries += 1;
    }

    NotifyTui::complete_task_step(&handle);
    
    // Now we need to create a disk to put in there from the supplied generic
    debug!("Bootstrapping the new disk...");
    let bootstrapped = T::bootstrap(blank_disk.disk_file(), next_open_disk)?;

    NotifyTui::complete_task_step(&handle);
    
    // The disk has now bootstrapped itself, we are done here.
    // We already locked earlier, so this can't be poisoned, unless maybe making the disks also panicked?
    if let Ok(mut inner) = GLOBAL_POOL.get().expect("Pool has to be set up before we can make disks.").try_lock() {
        inner.header.highest_known_disk += 1;
    } else {
        // Poisoned again! We're probably in really bad shape. Just give up.
        // In theory this cant even get poisoned at this point due to bootstrapping happening
        // in the same thread but whatever, compiler doesn't know ig.
        panic!("Poisoned on disk bootstrapping!");
    }
    

    debug!("Done adding new disk.");
    NotifyTui::finish_task(handle);
    Ok(bootstrapped)
}

/// Grabs the root inode block
fn pool_get_root_directory() -> Result<DirectoryBlock, DriveError> {
    // Root directory should always be at disk 1 block 2. We just assume that to be the case.
    // Why do we have a root inode that points to the root directory when its always in a static location?
    // Beats me, I forgot why I did that.

    let root_pointer: DiskPointer = DiskPointer {
        disk: 1,
        block: 2,
    };

    // Get the root directory block
    let block_reader: RawBlock = CachedBlockIO::read_block(root_pointer)?;
    let block = DirectoryBlock::from_block(&block_reader);

    Ok(block)
}

/// Grabs the root inode location, duh
fn pool_get_root_inode_location() -> InodeLocation {
    let pointer = DiskPointer {
        disk: 1,
        block: 1,
    };
    InodeLocation::new(pointer, 0)
}

/// Constructs a directory item with info about the root.
fn pool_get_root_directory_item() -> DirectoryItem {
    // The name of the root directory entry is always the delimiter.
    static DELIMITER: char = std::path::MAIN_SEPARATOR;
    DirectoryItem {
        flags: DirectoryItemFlags::IsDirectory,
        name_length: 0,
        name: DELIMITER.into(),
        location: pool_get_root_inode_location(),
    }
}