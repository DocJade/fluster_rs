// Pool level block allocations

use log::debug;

use crate::pool::{
    disk::{
        drive_struct::{DiskType, FloppyDrive, FloppyDriveError, JustDiskType},
        generic::{
            block::{allocate::block_allocation::BlockAllocation, block_structs::RawBlock, crc::add_crc_to_block},
            generic_structs::pointer_struct::DiskPointer, io::cache::cache_io::CachedBlockIO,
        },
        standard_disk::standard_disk_struct::StandardDisk,
    },
    pool_actions::pool_struct::{Pool, GLOBAL_POOL},
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
        go_find_free_pool_blocks(blocks, false, false)
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
    /// You can optionally also set the CRC on the new empty blocks, which is useful for
    /// new file blocks.
    /// 
    /// Will add new disks if needed.
    /// 
    /// May swap disks, will not return to where it started.
    /// 
    /// Returns disk pointers for the newly reserved blocks, or a disk error.
    pub fn find_and_allocate_pool_blocks(blocks: u16, add_crc: bool) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        // This is just an abstraction to force a different function name, even though
        // the function it calls is the same as find_free_pool_blocks()
        go_find_free_pool_blocks(blocks, true, add_crc)
    }

    /// Frees a block from a disk in the pool.
    /// 
    /// All blocks must come from same disk.
    /// 
    /// Returns how many blocks were freed.
    /// 
    /// Will destroy any data currently in that block.
    pub fn free_pool_block_from_disk(blocks: &[DiskPointer]) -> Result<u16, FloppyDriveError> {
        go_deallocate_pool_block(blocks)
    }
}

fn go_find_free_pool_blocks(blocks: u16, mark: bool, add_crc: bool) -> Result<Vec<DiskPointer>, FloppyDriveError> {
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
                free_blocks.append(&mut block_indexes_to_pointers(&ok, disk_to_check));

                // Allocate those blocks if needed.
                if mark {
                    let _ = disk.allocate_blocks(&ok)?;
                    // We also need to update the global pool to say these were marked as used, otherwise we would never know.
                    // Trust me I found out the hard way.
                    debug!("Updating the pool's free block count...");
                    debug!("Locking GLOBAL_POOL...");
                    GLOBAL_POOL
                        .get()
                        .expect("single threaded")
                        .try_lock()
                        .expect("single threaded")
                        .header
                        .pool_standard_blocks_free -= ok.len() as u16;
                }

                // Add crc to blocks if requested.
                // You must have already marked the new block.
                if add_crc && mark {
                    write_empty_crc(&ok, disk.number)?;
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
                free_blocks.append(&mut block_indexes_to_pointers(&blockie_doos, disk_to_check));

                // Allocate those blocks if needed.
                if mark {
                    let _ = disk.allocate_blocks(&blockie_doos)?;
                    // We also need to update the global pool to say these were marked as used, otherwise we would never know.
                    // Trust me I found out the hard way.
                    debug!("Updating the pool's free block count...");
                    debug!("Locking GLOBAL_POOL...");
                    GLOBAL_POOL
                        .get()
                        .expect("single threaded")
                        .try_lock()
                        .expect("single threaded")
                        .header
                        .pool_standard_blocks_free -= blockie_doos.len() as u16;
                }
                // Add crc to blocks if requested
                // You must have already marked the new block.
                if add_crc && mark {
                    write_empty_crc(&blockie_doos, disk.number)?;
                }

                // Waiter! Waiter! More disks please!
                disk_to_check += 1;
                continue;
            }
        }
    }

    // Now that we have allocated, the most probable disk is the last disk we got blocks from
    GLOBAL_POOL
        .get()
        .expect("Single threaded")
        .try_lock()
        .expect("Single threaded")
        .header
        .disk_with_next_free_block = disk_to_check;



    // We will sort the resulting vector to make to group the disks together, this will
    // reduce swapping.

    free_blocks.sort_unstable_by_key(|pointer| (pointer.disk, pointer.block));

    debug!("Allocation complete.");
    Ok(free_blocks)
}

// helper
fn block_indexes_to_pointers(blocks: &Vec<u16>, disk: u16) -> Vec<DiskPointer> {
    let mut result: Vec<DiskPointer> = Vec::new();
    for block in blocks {
        result.push(DiskPointer { disk, block: *block });
    }
    result
}

/// Sometimes we need a new block that is empty, but we still need to have the crc set.
/// Assumes block are already marked.
/// Will not swap disks.
/// Assumes blocks are for the disk currently in the drive.
/// 
/// This method only works on standard disks
fn write_empty_crc(blocks: &[u16], disk: u16) -> Result<(), FloppyDriveError> {
    // These new blocks do not have their CRC set, we need to just write empty blocks to them to set the crc.
    let mut empty_data: [u8; 512] = [0_u8; 512];
    // CRC that sucker
    add_crc_to_block(&mut empty_data);

    // Make block to write, must update inside of loop.
    for block in blocks {
        let block_origin: DiskPointer = DiskPointer {
            disk,
            block: *block,
        };
        let empty_raw_block: RawBlock = RawBlock {
            block_origin,
            data: empty_data,
        };

        CachedBlockIO::update_block(&empty_raw_block, JustDiskType::Standard)?;
    }

    // All of the blocks now have a empty block with a crc on it.
    Ok(())
}

fn go_deallocate_pool_block(blocks: &[DiskPointer]) -> Result<u16, FloppyDriveError> {
    // Make sure all of the blocks came from the same disk
    let starter: DiskPointer = *blocks.first().expect("Why are we getting 0 blocks?");
    let mut extracted_blocks: Vec<u16> = Vec::new();
    for block in blocks {
        // Make sure all blocks are from the same disk.
        assert_eq!(starter.disk, block.disk);
        extracted_blocks.push(block.block);
    }

    // Remove the blocks from the cache if they exist
    todo!();

    // Go zero out the blocks on the disk
    todo!();

    // Now go to that disk and free the blocks
    let mut disk: StandardDisk = match FloppyDrive::open(starter.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("Block allocations must be on standard disks!"),
    };
    
    let blocks_freed = disk.free_blocks(&extracted_blocks)?;

    // If the current disk in the pool is higher than the blocks we just freed, we need to move back
    // the search start for finding new free blocks.

    let probable_disk = GLOBAL_POOL
        .get()
        .expect("Single threaded")
        .try_lock()
        .expect("Single threaded")
        .header
        .disk_with_next_free_block;

    if probable_disk > disk.number {
        // It's higher, we need to move the pool back.
        GLOBAL_POOL
            .get()
            .expect("Single threaded")
            .try_lock()
            .expect("Single threaded")
            .header
            .disk_with_next_free_block = disk.number;
    }

    // Return the number of blocks freed.
    Ok(blocks_freed)
    
}