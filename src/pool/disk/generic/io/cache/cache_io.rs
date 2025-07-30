// External interaction with the block cache

use crate::pool::disk::{drive_struct::{FloppyDriveError, JustDiskType}, generic::{block::block_structs::RawBlock, disk_trait::GenericDiskMethods, generic_structs::pointer_struct::DiskPointer}};

//
// =========
// Structs
// =========
//


/// Struct for implementing cache methods on.
/// Holds no information, this is just for calling.
pub struct CachedBlockIO {
   // m tea
}

// Cache methods
impl CachedBlockIO {
    /// Sometimes you need to forcibly write a disk during initialization procedures, so we need a bypass.
    /// 
    /// !! == DANGER == !!
    /// 
    /// This function should ONLY be used when initializing disks, since this does not properly update the cache.
    /// The information written with this function will not be written to cache, nor will the information about this
    /// disk be flushed from the cache.
    /// 
    /// This function also does not update the allocation table.
    /// 
    /// You better know what you're doing.
    /// 
    /// !! == DANGER == !!
    /// 
    /// You must pass in the disk to write to.
    pub fn forcibly_write_a_block<T: GenericDiskMethods>(raw_block: &RawBlock, disk_to_write_on: &mut T) -> Result<(), FloppyDriveError> {
        go_force_write_block(raw_block, disk_to_write_on)
    }

    /// Reads in a block from disk, attempts to read it from the cache first.
    /// 
    /// You must specify the type of disk the block is being read from, otherwise you cannot guarantee that you
    /// received data from the correct disk type.
    pub fn read_block(block_origin: DiskPointer, expected_disk_type: JustDiskType) -> Result<RawBlock, FloppyDriveError> {
        go_read_cached_block(block_origin, expected_disk_type)
    }
    /// Writes a block to disk. Adds newly written block to cache.
    /// 
    /// You must specify the type of disk the block is being written to, otherwise you cannot guarantee that you
    /// wrote to the correct disk.
    pub fn write_block(raw_block: &RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
        go_write_cached_block(raw_block, disk_number, expected_disk_type)
    }
    /// Updates pre-existing block on disk, updates cache.
    /// 
    /// You must specify the type of disk the block is being written to, otherwise you cannot guarantee that you
    /// wrote to the correct disk.
    pub fn update_block(raw_block: &RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
        go_update_cached_block(raw_block, disk_number, expected_disk_type)
    }
}

//
// =========
// CachedBlockIO functions
// =========
//


// This function also updates the block order after the read.
fn go_read_cached_block(block_location: DiskPointer, expected_disk_type: JustDiskType) -> Result<RawBlock, FloppyDriveError> {
    // Grab the block from the cache if it exists.
    
    if let Some(index) = BlockCache::find_block(&block_location, &expected_disk_type) {
        // It was in the cache! Return the block...
        let cached = &BLOCK_CACHE.lock().expect("Single thread")[index];
        let constructed: RawBlock = RawBlock {
            block_index: cached.block_origin.block,
            originating_disk: Some(cached.block_origin.disk),
            data: cached.data.clone().try_into().expect("This should be 512 bytes."),
        };
        // Update the hit count
        CACHE_STATISTICS.lock().expect("Single threaded").record_hit(true);

        return Ok(constructed);
    }

    // The block was not in the cache, we need to go get it old-school style.
    let disk = FloppyDrive::open(block_location.disk)?;
    // make sure that is the right type
    assert_eq!(disk, expected_disk_type);

    // Just in case...
    assert_ne!(disk, JustDiskType::Blank);
    assert_ne!(disk, JustDiskType::Unknown);

    // Now read that block
    let read_block = disk.checked_read(block_location.block)?;
    
    // Add it to the cache
    todo!();
    BlockCache::update_or_add_block(block_location, expected_disk_type, read_block.data.to_vec());

    // Update the hit count
    CACHE_STATISTICS.lock().expect("Single threaded").record_hit(false);

    // Return the block.
    return Ok(read_block);
}

fn go_write_cached_block(raw_block: &RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
    // Write a block to the disk, also updating the cache with the block (or adding it if it does not yet exist.)
    let mut disk = FloppyDrive::open(disk_number)?;
    
    // Make sure this is the write one...
    assert_eq!(disk, expected_disk_type);

    // Just in case...
    assert_ne!(disk, JustDiskType::Blank);
    assert_ne!(disk, JustDiskType::Unknown);

    // Write the block.
    disk.checked_write(raw_block)?;

    // Now update the cache with the updated block.
    todo!();
    let extract: DiskPointer = DiskPointer {
        disk: disk_number,
        block: raw_block.block_index,
    };

    BlockCache::update_or_add_block(extract, expected_disk_type, raw_block.data.to_vec());

    Ok(())
}

fn go_update_cached_block(raw_block: &RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
    // Update like windows, but better idk this joke sucks
    let mut disk = FloppyDrive::open(disk_number)?;
    
    // Make sure this is the write one...
    assert_eq!(disk, expected_disk_type);

    // Just in case...
    assert_ne!(disk, JustDiskType::Blank);
    assert_ne!(disk, JustDiskType::Unknown);

    // Write the block.
    disk.checked_update(raw_block)?;

    // Now update the cache with the updated block.
    todo!();
    let extract: DiskPointer = DiskPointer {
        disk: disk_number,
        block: raw_block.block_index,
    };

    BlockCache::update_or_add_block(extract, expected_disk_type, raw_block.data.to_vec());
    Ok(())
}

fn go_force_write_block<T: GenericDiskMethods>(raw_block: &RawBlock, disk_to_write_on: &mut T) -> Result<(), FloppyDriveError> {
    // Since we are writing directly to this disk without being able to check if its the right disk, we must assume that the
    // caller knows what they're doing and is handling loading in the correct disk for us. There aren't any safeguards we can put in at this point.
    // Since we are force writing, we will invalidate all items in the cache from that disk. Chances are there won't be
    // anything there in the first place, since this should only be used on disk initialization.

    // This will fail on unknown and blank disks, you must first spoof the disk type before sending it in here.

    disk_to_write_on.unchecked_write_block(raw_block)?;
    Ok(())
}