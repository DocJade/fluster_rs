// Pool level block allocations

use log::debug;

use crate::pool::{
    disk::{
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock,
                crc::add_crc_to_block
            },
            generic_structs::pointer_struct::DiskPointer,
            io::cache::{
                cache_io::CachedBlockIO,
                cached_allocation::CachedAllocationDisk
            }
        },
        standard_disk::standard_disk_struct::StandardDisk,
    },
    pool_actions::pool_struct::{
        Pool,
        GLOBAL_POOL
    },
};

impl Pool {
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
        go_find_free_pool_blocks(blocks, add_crc)
    }

    /// Frees a block from a disk in the pool.
    /// 
    /// It's not required, but you should sort the order of the blocks to
    /// reduce drive seeking.
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

fn go_find_free_pool_blocks(blocks: u16, add_crc: bool) -> Result<Vec<DiskPointer>, FloppyDriveError> {
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
        // Since we use a mix of real and fake disks in here, we need to have a type that we can use for our allocation
        // methods. So we will box it up. Yes this is kinda evil.
        let mut disk: Box<dyn BlockAllocation>;
        // Check if the disk we are about to load is out of range
        if disk_to_check > new_highest_disk {
            debug!("Ran out of room, creating new disk...");
            // We need to make this disk before trying to allocate blocks on it.
            let new_disk: StandardDisk = Pool::new_disk::<StandardDisk>()?;
            disk = Box::new(new_disk);
            // increment the highest known disk
            new_highest_disk += 1;
        } else {
            // We are loading a pre-existing disk, hit up the cache for it.
            let new_disk: CachedAllocationDisk = CachedAllocationDisk::open(disk_to_check)?;
            disk = Box::new(new_disk);
        };

        // Check if this disk has enough room.
        // we will grab all we can.
        match disk.find_free_blocks(blocks - free_blocks.len() as u16) {
            Ok(ok) => {
                debug!("Found the last {} blocks we needed on disk {}!", ok.len(), disk_to_check);
                // We were able to allocate all of the blocks we asked for!
                // We're done!
                free_blocks.append(&mut block_indexes_to_pointers(&ok, disk_to_check));

                // Allocate those blocks.
                let _ = disk.allocate_blocks(&ok)?;

                // Now we drop the disk to make it flush the new allocation table.
                drop(disk);

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

                // Add crc to blocks if requested.
                if add_crc {
                    debug!("CRC requested, adding...");
                    write_empty_crc(&ok, disk_to_check)?;
                }

                break;
            }
            Err(amount) => {
                // There wasn't enough blocks free on this disk, but we can allocate at least `amount`
                debug!("Still need more blocks, disk {disk_to_check} only had {amount} blocks free.");
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
                
                let _ = disk.allocate_blocks(&blockie_doos)?;
                // Now we drop the disk to make it flush the new allocation table.
                drop(disk);

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
                
                // Add crc to blocks if requested
                if add_crc {
                    debug!("CRC requested, adding...");
                    write_empty_crc(&blockie_doos, disk_to_check)?;
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

        CachedBlockIO::update_block(&empty_raw_block)?;
    }

    // All of the blocks now have a empty block with a crc on it.
    Ok(())
}

fn go_deallocate_pool_block(blocks: &[DiskPointer]) -> Result<u16, FloppyDriveError> {
    // We assume the blocks are pre-sorted to reduce disk seeking.

    // Make sure all of the blocks came from the same disk
    let starter: DiskPointer = *blocks.first().expect("Why are we getting 0 blocks?");
    let mut extracted_blocks: Vec<u16> = Vec::new();
    for block in blocks {
        // Are the disk numbers the same?
        assert_eq!(starter.disk, block.disk);
        // Also hold onto the block number, need it for disk call.
        extracted_blocks.push(block.block);
    }

    // Remove the blocks from the cache if they exist.
    for block in blocks {
        CachedBlockIO::remove_block(block);
    }

    // Go zero out the blocks on the disk, just to be safe.
    // We will bypass the cache.
    
    for block in blocks {
        let empty: RawBlock = RawBlock {
            block_origin: *block,
            data: [0_u8; 512],
        };
        // Zero em out with the cache.
        CachedBlockIO::forcibly_write_a_block(&empty)?;
    }
    
    // Now go to and free the blocks from the allocation table.
    let mut disk: CachedAllocationDisk = CachedAllocationDisk::open(starter.disk)?;
    let blocks_freed = disk.free_blocks(&extracted_blocks)?;

    // Drop it to flush the updated header to cache.
    drop(disk);

    // If the current disk in the pool marked with free blocks is higher than the blocks we just freed,
    // we need to move back the search start for finding new free blocks.

    let probable_disk = GLOBAL_POOL
        .get()
        .expect("Single threaded")
        .try_lock()
        .expect("Single threaded")
        .header
        .disk_with_next_free_block;

    if probable_disk > starter.disk {
        // It's higher, we need to move the pool back.
        GLOBAL_POOL
            .get()
            .expect("Single threaded")
            .try_lock()
            .expect("Single threaded")
            .header
            .disk_with_next_free_block = starter.disk;
    }

    // Update the free count, since new blocks are available.
    GLOBAL_POOL
    .get()
    .expect("single threaded")
    .try_lock()
    .expect("single threaded")
    .header
    .pool_standard_blocks_free += blocks_freed;

    // Return the number of blocks freed.
    Ok(blocks_freed)
    
}