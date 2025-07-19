// Inode this was going somewhere...

// Imports

// Implementations

// Functions

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeLocation}, standard_disk_struct::StandardDisk}};


/// Add an inode to the never ending inode chain.
/// Returns where the inode ended up.
/// This function will create new disks if needed.
fn add_inode(inode: Inode) -> Result<InodeLocation, FloppyDriveError> {
    // We need the start of the inode train, we have to work off from disk 1.
    let mut current_disk = match FloppyDrive::open(1)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("Disk 1 MUST be a standard disk."),
    };

    // From that disk, we shall load block 1, which has the origin inode block.
    // This is where we will start searching for room.
    let mut current_block: InodeBlock = InodeBlock::from_block(&current_disk.checked_read(1)?);
    let mut current_block_number: u16 = 1;
    // For when we eventually find a spot.
    let mut inode_offset: u16;

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
            current_disk = match FloppyDrive::open(1)? {
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