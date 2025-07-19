// Inode this was going somewhere...

// Imports

// Implementations

// Functions

use log::debug;

use crate::pool::{disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeLocation}, standard_disk_struct::StandardDisk}}, pool_struct::{Pool, GLOBAL_POOL}};

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
    fn fast_add_inode(inode: Inode) -> Result<InodeLocation, FloppyDriveError> {
        debug!("Fast adding inode...");
        // Get the pool's latest inode disk
        let start_disk: u16 = GLOBAL_POOL.get().expect("Single thread").lock().expect("Single thread").header.disk_with_latest_inode_write.clone();

        let result = go_add_inode(inode, start_disk)?;

        // Update the pool with new successful write.
        GLOBAL_POOL.get().expect("Single thread").lock().expect("Single thread").header.disk_with_latest_inode_write = result.disk.expect("Should always return a disk number");
        
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
    fn add_inode(inode: Inode) -> Result<InodeLocation, FloppyDriveError> {
        debug!("Adding inode, starting from disk 1...");
        debug!("Adding inode, starting from disk 1...");
        // Start from the origin.
        let start_disk: u16 = 1;
        
        let result = go_add_inode(inode, start_disk)?;
        
        // Update the pool with new successful write.
        GLOBAL_POOL.get().expect("Single thread").lock().expect("Single thread").header.disk_with_latest_inode_write = result.disk.expect("Should always return a disk number");
        
        // all done
        Ok(result)
    }
    /// Add an inode to the never ending inode chain.
    /// 
    /// This method will attempt to add an inode to the currently
    /// inserted disk, then traverse upwards from there.
    /// 
    /// This is fast, but wasteful.
    /// 
    /// If allocating an inode on the current disk fails,
    /// we fallback to fast_add_inode.
    /// 
    /// This function will traverse to the next blocks in the chain,
    /// and create new disks if needed.
    ///
    /// Returns where the inode ended up.
    fn greedy_add_inode(inode: Inode, current_disk: u16) -> Result<InodeLocation, FloppyDriveError> {
        // Just go for it, yolo
        debug!("Greedily adding inode...");
        go_add_inode(inode, current_disk)
        // Does not update the pool to set where the last successful add was.
    }
}


fn go_add_inode(inode: Inode, start_disk: u16) -> Result<InodeLocation, FloppyDriveError> {
    // We need the start of the inode train, we will start from the provided disk.

    let mut current_disk = match FloppyDrive::open(start_disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("Start disk MUST be a standard disk."),
    };

    // From that disk, we shall load block 1, which has the origin inode block.
    // This is where we will start searching for room.
    let mut current_block: InodeBlock = InodeBlock::from_block(&current_disk.checked_read(1)?);
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
        let DiskPointer {
            disk,
            block,
        } = get_next_block(current_block, current_disk.number, current_block_number);
        // Go there
        if disk != current_disk.number {
            // On another disk.
            current_disk = match FloppyDrive::open(disk)? {
                DiskType::Standard(standard_disk) => standard_disk,
                _ => panic!("New inode block must be on a standard disk!"),
            };
        }
        current_block = InodeBlock::from_block(&current_disk.checked_read(block)?);
        current_block_number = block;
        // start over!
        continue;
    }

    // All done! Now we can return where that inode eventually ended up
    Ok(
        InodeLocation {
            disk: Some(current_disk.number),
            block: current_block_number,
            offset: inode_offset,
        }
    )
}

fn get_next_block(block: InodeBlock, current_disk_number: u16, current_block_number: u16) -> DiskPointer {
    // Extract the pointer
    if let Some(ok) = block.next_block(current_disk_number) {
        // Sweet, it exists already.
        return ok
    }
    // Time to make a new inode block, and update the pointer on the previous block
    todo!()
    // todo: InodeBlock::update_destination()
    // Somehow call to the pool level to get a new block
    // Somehow update the pool when a disk is filled (full allocate) or gains room (now has free blocks) to change the next free block
}


/// We need a new inode block, we will reach upwards and get a new block made for us.
fn new_inode_block() -> DiskPointer {
    // Planned implementation is to add a "find free block" implementation to the pool which will
    // automatically go find the next free block, or make a new disk and hand us that.
    // It will return a DiskPointer.
    // In here, we will know if its a new disk, because if we are trying to make a new inode block, we are on
    // the final inode block on this disk, so we are either going to make a new block on this disk, or point to
    // block 1 on the new disk.
    todo!()
}