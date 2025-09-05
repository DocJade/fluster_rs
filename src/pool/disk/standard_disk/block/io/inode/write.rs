// Inode this was going somewhere...

// Imports

// Implementations

// Functions

use log::trace;

use crate::{
    error_types::{
        block::BlockManipulationError,
        drive::DriveError
    },
    pool::{
        disk::{
            generic::{
                block::block_structs::RawBlock,
                generic_structs::pointer_struct::DiskPointer,
                io::cache::cache_io::CachedBlockIO,
            },
            standard_disk::block::inode::inode_struct::{
                    Inode,
                    InodeBlock,
                    InodeLocation
                },
        },
        pool_actions::pool_struct::{
            Pool,
            GLOBAL_POOL
        },
    }
};


// The pool MUST exist for inodes to be created.
macro_rules! get_pool {
    () => {
        if let Ok(innards) = GLOBAL_POOL.get().expect("There has to be a global pool at this point.").try_lock() {
            innards
        } else {
            // Cannot do inode stuff with dying pool, dying pools need to just shut down immediately.
            panic!("A poisoned pool cannot have inode operations performed against it!");
        }
    };
}




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
    pub fn fast_add_inode(inode: Inode) -> Result<InodeLocation, DriveError> {
        trace!("Fast adding inode...");
        // Get the pool's latest inode disk
        let start_pointer: DiskPointer = get_pool!().header.latest_inode_write;

        // load in that block
        let da_reader: RawBlock = CachedBlockIO::read_block(start_pointer)?;
        let start_block: InodeBlock = InodeBlock::from_block(&da_reader);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = result.pointer;

        // Update the pool with new successful write.
        {
            let mut pool = get_pool!();
            pool.header.latest_inode_write = success_write_pointer;
        }

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
    pub fn add_inode(inode: Inode) -> Result<InodeLocation, DriveError> {
        trace!("Adding inode, starting from disk 1...");
        // Start from the origin.
        let start_pointer: DiskPointer = DiskPointer { disk: 1, block: 1 };

        let da_reader: RawBlock = CachedBlockIO::read_block(start_pointer)?;

        let start_block: InodeBlock = InodeBlock::from_block(&da_reader);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = result.pointer;

        // Update the pool with new successful write.
        {
            let mut pool = get_pool!();
            pool.header.latest_inode_write = success_write_pointer;
        }

        // all done
        Ok(result)
    }
}

fn go_add_inode(inode: Inode, start_block: InodeBlock) -> Result<InodeLocation, DriveError> {
    // We will start from the provided block.

    
    // For when we switch disks (abstracted away but hehe!)
    let mut current_block_number: u16 = start_block.block_origin.block;
    let mut current_disk: u16 = start_block.block_origin.disk;
    let mut current_block: InodeBlock = start_block;
    // For when we eventually find a spot.
    let inode_offset: u16;

    // Now we loop, looking for room.
    loop {
        // Does the current block have room?
        match current_block.try_add_inode(inode) {
            Ok(ok) => {
                // We've got a spot!
                inode_offset = ok;
                break;
            }
            Err(error) => match error {
                BlockManipulationError::OutOfRoom => {
                    // Not enough room on this block, go fish.
                }
                BlockManipulationError::Impossible => {
                    // Impossible offsets should never happen.
                    // Cant keep going with logic errors.
                    panic!("Impossible inode offset. {inode:#?}");
                },
                BlockManipulationError::NotFinalBlockInChain | BlockManipulationError::NotPresent => {
                    // Not possible here, inodes dont care about being the final block,
                    // and adding doesnt check if an item is present.
                    panic!("Apparently inodes DO care!");
                },
            },
        }
        // There wasn't enough room, proceed to the next block.
        let pointer_to_next_block: DiskPointer = get_next_block(current_block.clone())?;
        // Go there
        // We always re-open the disk to get the freshest allocation table.
        current_disk = pointer_to_next_block.disk;

        let reader: RawBlock = CachedBlockIO::read_block(pointer_to_next_block)?;
        current_block = InodeBlock::from_block(&reader);
        current_block_number = pointer_to_next_block.block;
        // start over!
        continue;
    }

    // The inode has now been added to the block, we must write this to disk before continuing.
    let block_to_write: RawBlock = current_block.to_block();
    // We are updating, because how would we be writing back to a block that was not allocated when we read it?
    CachedBlockIO::update_block(&block_to_write)?;

    // All done! Now we can return where that inode eventually ended up
    let pointer: DiskPointer = DiskPointer {
        disk: current_disk,
        block: current_block_number,
    };

    let new_location: InodeLocation = InodeLocation::new(pointer, inode_offset);

    Ok(new_location)
}

fn get_next_block(current_block: InodeBlock) -> Result<DiskPointer, DriveError> {
    // Extract the pointer
    if let Some(ok) = current_block.next_block() {
        // Sweet, it exists already.
        return Ok(ok);
    }

    // Time to make a new inode block
    // Why is there a second function for that? idk i felt like it
    // This writes the new block.
    let new_block_location: DiskPointer = make_new_inode_block()?;

    // Now we just need to update the block we were called on.
    let mut the_cooler_inode = current_block;
    the_cooler_inode.new_destination(new_block_location);
    let please_let_me_hit = the_cooler_inode.to_block();

    // Update that block G
    CachedBlockIO::update_block(&please_let_me_hit)?;

    // return the pointer to the next block.
    Ok(new_block_location)
}

/// We need a new inode block, we will reach upwards and get a new block made for us.
fn make_new_inode_block() -> Result<DiskPointer, DriveError> {
    // Ask the pool for a new block pwease
    // No need for crc since we'll be overwriting it immediately.
    let ask_nicely = Pool::find_and_allocate_pool_blocks(1, false)?;
    // And you shall receive.
    let new_block_location = ask_nicely[0];

    // New block to throw there
    let new_block: InodeBlock = InodeBlock::new(new_block_location);
    let but_raw: RawBlock = new_block.to_block();

    // Write it Ralph!
    // I'm gonna write it!
    // New block, so standard write.
    CachedBlockIO::update_block(&but_raw)?;

    // Now throw it back, I mean the pointer
    Ok(new_block_location)
}
