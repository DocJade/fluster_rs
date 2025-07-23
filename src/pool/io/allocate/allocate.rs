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
        go_find_free_pool_blocks(blocks)
    }
}

fn go_find_free_pool_blocks(blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    debug!("Attempting to allocate {blocks} blocks across the pool...");

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
        let disk: StandardDisk;
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
                free_blocks.append(&mut block_indexes_to_pointers(ok, disk_to_check));
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
                free_blocks.append(&mut block_indexes_to_pointers(blockie_doos, disk_to_check));
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
