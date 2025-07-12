// Interacting with the pool

use crate::disk::{disk_struct::{Disk, DiskError}, pool::pool_struct::PoolInfo};

impl PoolInfo {
    /// Sync information about the pool to disk
    pub fn sync(self) -> Result<(), ()> {
        sync(self)
    }
    /// Read in pool information from disk
    pub fn initialize() -> PoolInfo {
        initialize()
    }
}


/// Sync information about the pool to disk
fn sync(pool: PoolInfo) -> Result<(), ()> {
    // Get the root disk so we can write pool info to it
    let root_disk = Disk::prompt_for_disk(0).unwrap();

    // Update the header.
    let mut new_header = root_disk.header;

    // What's the highest disk we've seen?
    new_header.highest_known_disk = pool.highest_known_disk;


    // Write the header to disk
    let header_block = new_header.to_disk_block();
    root_disk.write_block(header_block);

    Ok(())
}






/// Read in pool information from disk
fn initialize() -> PoolInfo {
    // When we are initializing the pool, we assume full reign over the connected disk.
    // But you should only initialize the pool on startup, and then never again. 
    // TODO: Do we need to enforce this?

    // First we need to load in the root disk.
    let root_disk: Disk;
    
    // Now there are a few cases to handle here:
    
    // Did the user insert the wrong disk?
    // Does no disk 0 exist yet?
    
    // TODO: Move this loop into its own function to keep things clean.
    loop {
        root_disk = match Disk::prompt_for_disk(0) {
            Ok(ok) => ok, // That was disk 0.
            Err(err) => {
                // Something is wrong with the disk
                if err == DiskError::Uninitialized {
                    // This may be the first time the user has used the filesystem.
                    let response = rprompt::prompt_reply("That disk is blank, would you like to create a root disk?. y/n: ").unwrap();
                    if response.to_lowercase().contains('y') {
                        // Time for a new disk.
                        Disk::create(0).unwrap();
                        println!("Root disk created.")
                    }
                    continue;
                } else {
                    // Something else is wrong with the disk, may be corruption.
                    todo!("{err}")
                }
            },
        };
        // We now have a disk
        break
    }

    // Using that root disk, lets fill in our Pool data.

    // The root disk contains the highest known disk.
    let highest_known_disk: u16 = root_disk.header.highest_known_disk;



    // Done!
    return PoolInfo {
        highest_known_disk,
    };

}