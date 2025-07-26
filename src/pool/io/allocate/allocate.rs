// Pool level block allocations

use log::debug;

use crate::pool::{
    disk::{
        drive_struct::{DiskType, FloppyDrive, FloppyDriveError},
        generic::{
            block::allocate::block_allocation::BlockAllocation,
            generic_structs::pointer_struct::DiskPointer,
        },
        standard_disk::standard_disk_struct::StandardDisk,
    },
    pool_actions::pool_struct::{GLOBAL_POOL, Pool},
};

impl Pool {
    /// Finds blocks across the entire pool.
    /// 
    /// == WARNING ==
    /// 
    /// You should really only be calling this for singular blocks, or when you know that you will write all of these blocks
    /// without possibly allocating another one. Since the blocks returned are not marked as allocated yet, in theory while you
    /// are writing to them, you may call this function again form another method (think expanding inodes) which would use one of
    /// the blocks that you will be assuming are free. If you need to know that these blocks wont move, you should reserve them
    /// ahead of time with find_and_allocate_pool_blocks() !
    /// 
    /// == WARNING ==
    ///
    /// The blocks will be searched for only on Standard disks, all other allocations have to be done on the individual disk.
    ///
    /// This does not mark the blocks as allocated, it only finds them.
    ///
    /// If there are not enough blocks, new disks will be added as needed.
    ///
    /// May swap disks, will not return to where it started.
    ///
    /// Returns disk pointers for the found blocks, or a disk error.
    pub fn find_free_pool_blocks(blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        // We will not be marking the blocks as used.
        go_find_free_pool_blocks(blocks, false)
    }
    /// Finds blocks across the entire pool.
    /// 
    /// Can only allocate a maximum of 32MB worth of blocks in one go.
    /// If you are asking for that many, something is 100% wrong.
    /// Writes should be limited to 1MB so this should never happen.
    /// 
    /// If this fails, ur kinda cooked ngl, since now a bunch of random blocks have been
    /// allocated for no reason.
    /// 
    /// Searches standard disks, yada yada.
    /// 
    /// This will mark the blocks as allocated.
    /// 
    /// Will add new disks if needed.
    /// 
    /// May swap disks, will not return to where it started.
    /// 
    /// Returns disk pointers for the newly reserved blocks, or a disk error.
    pub fn find_and_allocate_pool_blocks(blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        // This is just an abstraction to force a different function name, even though
        // the function it calls is the same as find_free_pool_blocks()
        go_find_free_pool_blocks(blocks, true)
    }
}

fn go_find_free_pool_blocks(blocks: u16, mark: bool) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    debug!("Attempting to allocate {blocks} blocks across the pool...");
    debug!("We will _{}_ be marking the blocks as used.", if mark {"will"} else {"will not"});


    debug!("Locking GLOBAL_POOL...");
    let probable_disk = GLOBAL_POOL
        .get()
        .expect("Single threaded")
        .try_lock()
        .expect("Single threaded")
        .header
        .disk_with_next_free_block;
    let highest_disk = GLOBAL_POOL
        .get()
        .expect("Single threaded")
        .try_lock()
        .expect("Single threaded")
        .header
        .highest_known_disk;

    // First we open up the disk with the most recent successful pool allocation
    let mut disk_to_check = probable_disk;
    let mut new_highest_disk = highest_disk; // We may create new disks during the process.
    let mut free_blocks: Vec<DiskPointer> = Vec::new();

    // Make sure the highest disks and disk to check are valid
    assert_ne!(disk_to_check, u16::MAX);
    assert!(new_highest_disk >= 1); // A new pool has at least 2 disks.

    // Now we loop until we find enough free blocks.
    loop {
        let mut disk: StandardDisk;
        // Check if the disk we are about to load is out of range
        if disk_to_check > new_highest_disk {
            debug!("Ran out of room, creating new disk...");
            // We need to make this disk before trying to allocate blocks on it.
            disk = Pool::new_disk::<StandardDisk>()?;
            // increment the highest known disk
            new_highest_disk += 1;
        } else {
            // We are loading a pre-existing disk.
            disk = match FloppyDrive::open(disk_to_check)? {
                DiskType::Standard(standard_disk) => standard_disk,
                _ => {
                    // cant allocate to a non-standard disk, so we must ask for yet another disk.
                    disk_to_check += 1;
                    continue;
                }
            };
        }

        // Check if this disk has enough room.
        // we will grab all we can.
        match disk.find_free_blocks(blocks - free_blocks.len() as u16) {
            Ok(ok) => {
                // We were able to allocate all of the blocks we asked for!
                // We're done!
                free_blocks.append(&mut block_indexes_to_pointers(ok.clone(), disk_to_check));

                // Allocate those blocks if needed.
                if mark {
                    disk.allocate_blocks(&ok)?;
                }

                break;
            }
            Err(amount) => {
                // There wasn't enough blocks free on this disk, but we can allocate at least `amount`
                // Bail early if there's zero blocks
                if amount == 0 {
                    disk_to_check += 1;
                    continue;
                }

                let blockie_doos = disk
                    .find_free_blocks(amount)
                    .expect("We already asked how much room you had.");
                free_blocks.append(&mut block_indexes_to_pointers(blockie_doos.clone(), disk_to_check));

                // Allocate those blocks if needed.
                if mark {
                    disk.allocate_blocks(&blockie_doos)?;
                }

                // Waiter! Waiter! More disks please!
                disk_to_check += 1;
                continue;
            }
        }
    }

    // We will sort the resulting vector to make to group the disks together, this will
    // reduce swapping.

    free_blocks.sort_unstable_by_key(|pointer| (pointer.disk, pointer.block));

    debug!("Allocation complete.");
    Ok(free_blocks)
}

// helper
fn block_indexes_to_pointers(blocks: Vec<u16>, disk: u16) -> Vec<DiskPointer> {
    let mut result: Vec<DiskPointer> = Vec::new();
    for block in blocks {
        result.push(DiskPointer { disk, block });
    }
    result
}
