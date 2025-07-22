// Inode this was going somewhere...

// Imports

// Implementations

// Functions

use log::debug;

use crate::pool::{disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{block::block_structs::RawBlock, disk_trait::GenericDiskMethods, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeLocation}, standard_disk_struct::StandardDisk}}, pool_actions::pool_struct::{Pool, GLOBAL_POOL}};

// For the pool implementations, we do not use Self, as we might try to double mut it if the inode
// addition routine adds a new disk.
impl Pool {
    /// Add an inode to the never ending inode chain.
    /// 
    /// This method will look at the pool, and attempt adding an inode
    /// to the lowest disk with the most recent successful inode write.
    /// (Ignoring manual writes done outside of this method.)
    /// 
    /// This function will traverse to the next blocks in the chain,
    /// and create new disks if needed.
    ///
    /// Returns where the inode ended up.
    pub fn fast_add_inode(inode: Inode) -> Result<InodeLocation, FloppyDriveError> {
        debug!("Fast adding inode...");
        // Get the pool's latest inode disk
        debug!("Locking GLOBAL_POOL...");
        let start_pointer: DiskPointer = GLOBAL_POOL.get().expect("Single thread").try_lock().expect("Single thread").header.latest_inode_write;

        // load in that block
        let current_disk: StandardDisk = match FloppyDrive::open(start_pointer.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => panic!("Incoming inode block must be from a standard disk!"),
        };

        let start_block: InodeBlock = InodeBlock::from_block(&current_disk.checked_read(start_pointer.block)?);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = DiskPointer {
            disk: result.disk.expect("That function should return some here."),
            block: result.block,
        };

        // Update the pool with new successful write.
        debug!("Locking GLOBAL_POOL...");
        GLOBAL_POOL.get().expect("Single thread").try_lock().expect("Single thread").header.latest_inode_write = success_write_pointer;
        
        // all done
        Ok(result)
    }
    /// Add an inode to the never ending inode chain.
    /// 
    /// This method adds an inode to the pool, starting from
    /// the origin inode block on the origin disk.
    /// This may take a long time, but will ensure that the first
    /// available spot within the entire inode pool is used.
    /// 
    /// This function will traverse to the next blocks in the chain,
    /// and create new disks if needed.
    ///
    /// Returns where the inode ended up.
    pub fn add_inode(inode: Inode) -> Result<InodeLocation, FloppyDriveError> {
        debug!("Adding inode, starting from disk 1...");
        // Start from the origin.
        let start_pointer: DiskPointer = DiskPointer { disk: 1, block: 1 };

        // load in that block
        let current_disk: StandardDisk = match FloppyDrive::open(start_pointer.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => panic!("Incoming inode block must be from a standard disk!"),
        };

        let start_block: InodeBlock = InodeBlock::from_block(&current_disk.checked_read(start_pointer.block)?);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = DiskPointer {
            disk: result.disk.expect("That function should return some here."),
            block: result.block,
        };

        // Update the pool with new successful write.
        debug!("Locking GLOBAL_POOL...");
        GLOBAL_POOL.get().expect("Single thread").try_lock().expect("Single thread").header.latest_inode_write = success_write_pointer;
        
        // all done
        Ok(result)
    }
}


fn go_add_inode(inode: Inode, start_block: InodeBlock) -> Result<InodeLocation, FloppyDriveError> {
    // We will start from the provided block.

    // For when we switch disks
    let mut current_disk: StandardDisk = match FloppyDrive::open(start_block.block_origin.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("New inode block must be on a standard disk!"),
    };

    let mut current_block: InodeBlock = start_block;
    let mut current_block_number: u16 = 1;
    // For when we eventually find a spot.
    let inode_offset: u16;

    // Now we loop, looking for room.
    loop {
        // Does the current block have room?
        match current_block.try_add_inode(inode) {
            Ok(ok) => {
                // We've got a spot!
                inode_offset = ok;
                break
            }
            Err(error) => match error {
                InodeBlockError::NotEnoughSpace => {
                    // Not enough room on this block, go fish.
                },
                InodeBlockError::BlockIsFragmented => {
                    // Someday, we'll be able to request an inode defrag.
                    todo!("Inode defrag not written.")
                },
                InodeBlockError::InvalidOffset => todo!(),
            },
        }
        // There wasn't enough room, proceed to the next block.
        let pointer_to_next_block: DiskPointer = get_next_block(current_block.clone())?;
        // Go there
        // We always re-open the disk to get the freshest allocation table.
        current_disk = match FloppyDrive::open(pointer_to_next_block.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => panic!("New inode block must be on a standard disk!"),
        };

        current_block = InodeBlock::from_block(&current_disk.checked_read(pointer_to_next_block.block)?);
        current_block_number = pointer_to_next_block.block;
        // start over!
        continue;
    }
    
    // The inode has now been added to the block, we must write this to disk before continuing.
    let block_to_write: RawBlock = current_block.to_block(current_block_number);
    // We are updating, because how would we be writing back to a block that was not allocated when we read it?
    current_disk.checked_update(&block_to_write)?;

    // All done! Now we can return where that inode eventually ended up
    Ok(
        InodeLocation {
            disk: Some(current_disk.number),
            block: current_block_number,
            offset: inode_offset,
        }
    )
}

fn get_next_block(current_block: InodeBlock) -> Result<DiskPointer, FloppyDriveError> {
    // Extract the pointer
    if let Some(ok) = current_block.next_block() {
        // Sweet, it exists already.
        return Ok(ok);
    }

    // Time to make a new inode block
    // Why is there a second function for that? idk i felt like it
    let new_block_location: DiskPointer = make_new_inode_block()?;

    // New block made, we must update the old one to point to it, then return the new one again
    let mut disco = match FloppyDrive::open(new_block_location.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("The disk this block came from was non-standard?"),
    };

    // Make the new block to write
    let mut the_cooler_inode = current_block;
    the_cooler_inode.new_destination(new_block_location);
    let please_let_me_hit = the_cooler_inode.to_block(the_cooler_inode.block_origin.disk);

    // Write the updated block
    disco.checked_update(&please_let_me_hit)?;

    // return the pointer to the next block.
    Ok(new_block_location)
}


/// We need a new inode block, we will reach upwards and get a new block made for us.
fn make_new_inode_block() -> Result<DiskPointer, FloppyDriveError> {
    // Ask the pool for a new block pwease
    let ask_nicely = Pool::find_free_pool_blocks(1)?;
    // And you shall receive.
    let new_block_location = ask_nicely.first().expect("Asked for 1.");
    
    // New block to throw there
    let new_block: InodeBlock = InodeBlock::new();
    let but_raw: RawBlock = new_block.to_block(new_block_location.block);

    // Write it Ralph!
    let mut disk = match FloppyDrive::open(new_block_location.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("Non standard disk was given when asking for new block."),
    };
    
    // I'm gonna write it!
    // New block, so standard write.
    disk.checked_write(&but_raw)?;

    // Now throw it back, I mean the pointer
    Ok(*new_block_location)
}