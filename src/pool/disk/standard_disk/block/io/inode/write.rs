// Inode this was going somewhere...

// Imports

// Implementations

// Functions

use log::trace;

use crate::pool::{
    disk::{
        drive_struct::{FloppyDriveError, JustDiskType},
        generic::{
            block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO,
        },
        standard_disk::{
            block::inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeLocation}
        },
    },
    pool_actions::pool_struct::{Pool, GLOBAL_POOL},
};

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
        trace!("Fast adding inode...");
        // Get the pool's latest inode disk
        trace!("Locking GLOBAL_POOL...");
        let start_pointer: DiskPointer = GLOBAL_POOL
            .get()
            .expect("Single thread")
            .try_lock()
            .expect("Single thread")
            .header
            .latest_inode_write;

        // load in that block
        let da_reader: RawBlock = CachedBlockIO::read_block(start_pointer, JustDiskType::Standard)?;
        let start_block: InodeBlock = InodeBlock::from_block(&da_reader);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = DiskPointer {
            disk: result.disk.expect("That function should return some here."),
            block: result.block,
        };

        // Update the pool with new successful write.
        trace!("Locking GLOBAL_POOL...");
        GLOBAL_POOL
            .get()
            .expect("Single thread")
            .try_lock()
            .expect("Single thread")
            .header
            .latest_inode_write = success_write_pointer;

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
        trace!("Adding inode, starting from disk 1...");
        // Start from the origin.
        let start_pointer: DiskPointer = DiskPointer { disk: 1, block: 1 };

        let da_reader: RawBlock = CachedBlockIO::read_block(start_pointer, JustDiskType::Standard)?;

        let start_block: InodeBlock = InodeBlock::from_block(&da_reader);

        let result = go_add_inode(inode, start_block)?;
        // Where that ended up needs to be known
        let success_write_pointer: DiskPointer = DiskPointer {
            disk: result.disk.expect("That function should return some here."),
            block: result.block,
        };

        // Update the pool with new successful write.
        trace!("Locking GLOBAL_POOL...");
        GLOBAL_POOL
            .get()
            .expect("Single thread")
            .try_lock()
            .expect("Single thread")
            .header
            .latest_inode_write = success_write_pointer;

        // all done
        Ok(result)
    }
}

fn go_add_inode(inode: Inode, start_block: InodeBlock) -> Result<InodeLocation, FloppyDriveError> {
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
                InodeBlockError::NotEnoughSpace => {
                    // Not enough room on this block, go fish.
                }
                InodeBlockError::BlockIsFragmented => {
                    // Someday, we'll be able to request an inode defrag.
                    todo!("Inode defrag not written.")
                }
                InodeBlockError::InvalidOffset => todo!(),
            },
        }
        // There wasn't enough room, proceed to the next block.
        let pointer_to_next_block: DiskPointer = get_next_block(current_block.clone())?;
        // Go there
        // We always re-open the disk to get the freshest allocation table.
        current_disk = pointer_to_next_block.disk;

        let reader: RawBlock = CachedBlockIO::read_block(pointer_to_next_block, JustDiskType::Standard)?;
        current_block = InodeBlock::from_block(&reader);
        current_block_number = pointer_to_next_block.block;
        // start over!
        continue;
    }

    // The inode has now been added to the block, we must write this to disk before continuing.
    let block_to_write: RawBlock = current_block.to_block(current_block_number);
    // We are updating, because how would we be writing back to a block that was not allocated when we read it?
    CachedBlockIO::update_block(&block_to_write, current_disk, JustDiskType::Standard)?;

    // All done! Now we can return where that inode eventually ended up
    Ok(InodeLocation {
        disk: Some(current_disk),
        block: current_block_number,
        offset: inode_offset,
    })
}

fn get_next_block(current_block: InodeBlock) -> Result<DiskPointer, FloppyDriveError> {
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
    let please_let_me_hit = the_cooler_inode.to_block(the_cooler_inode.block_origin.block);

    // Update that block G
    CachedBlockIO::update_block(&please_let_me_hit, the_cooler_inode.block_origin.disk, JustDiskType::Standard)?;

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
    // I'm gonna write it!
    // New block, so standard write.
    CachedBlockIO::write_block(&but_raw, new_block_location.disk, JustDiskType::Standard)?;

    // Now throw it back, I mean the pointer
    Ok(*new_block_location)
}
