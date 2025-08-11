// External interaction with the block cache

use crate::pool::disk::{
    drive_struct::FloppyDriveError,
     generic::{block::block_structs::RawBlock,
        disk_trait::GenericDiskMethods,
        generic_structs::pointer_struct::DiskPointer,
        io::{
            cache::cache_implementation::{
                BlockCache, CachedBlock
            },
            checked_io::CheckedIO
        }
    },
    standard_disk::standard_disk_struct::StandardDisk
};

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
    /// This will ensure the correct disk is in the drive, and the header is properly up to date before
    /// writing anything.
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
    pub fn forcibly_write_a_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
        go_force_write_block(raw_block)
    }

    /// Attempts to read a block from the cache, does not load from disk if not present.
    /// 
    /// Returns the block if present, or None if absent.
    pub fn try_read(block_origin: DiskPointer) -> Option<RawBlock> {
        if let Some(cached) = BlockCache::try_find(block_origin) {
            // Was there!
            return Some(cached.into_raw())
        }
        // Missing.
        None
    }

    /// Reads in a block from disk, attempts to read it from the cache first.
    /// 
    /// Only works on standard disks.
    pub fn read_block(block_origin: DiskPointer) -> Result<RawBlock, FloppyDriveError> {
        go_read_cached_block(block_origin)
    }
    /// Writes a block to disk. Adds newly written block to cache.
    /// 
    /// Only works on standard disks.
    pub fn write_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
        go_write_cached_block(raw_block)
    }
    /// Updates pre-existing block on disk, updates cache.
    /// 
    /// Only works on standard disks.
    pub fn update_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
        go_update_cached_block(raw_block)
    }
    /// Get the hit-rate of the underlying cache
    pub fn get_hit_rate() -> f32 {
        BlockCache::get_hit_rate()
    }
    /// Sometimes you just need to remove a block from the cache, not even set it to zeros.
    pub fn remove_block(block_origin: &DiskPointer) {
        BlockCache::remove_item(block_origin)
    }
    /// Flush the entire cache to disk.
    pub fn flush() -> Result<(), FloppyDriveError> {
        // There are currently 3 tiers of cache.
        // ! If that changes, this must be updated !
        // ! or there will be unflushed data still !
        BlockCache::flush(0)?;
        BlockCache::flush(1)?;
        BlockCache::flush(2)
    }
}

//
// =========
// CachedBlockIO functions
// =========
//


// This function also updates the block order after the read.
fn go_read_cached_block(block_location: DiskPointer) -> Result<RawBlock, FloppyDriveError> {
    // Grab the block from the cache if it exists.
    
    if let Some(found_block) = BlockCache::try_find(block_location) {
        // It was in the cache! Return the block...
        return Ok(found_block.into_raw());
    }

    // The block was not in the cache, we need to go get it old-school style.
    let disk: StandardDisk = super::cache_implementation::disk_load_header_invalidation(block_location.disk)?;

    // Now read that block
    let read_block = disk.checked_read(block_location.block)?;
    
    // Add it to the cache
    BlockCache::add_or_update_item(CachedBlock::from_raw(&read_block))?;

    // Return the block.
    Ok(read_block)
}

fn go_write_cached_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
    // Write a block to the disk, also updating the cache with the block (or adding it if it does not yet exist.)

    // The cache expects the block's destination to be allocated already, so we will allocate it here.
    // We want to use the cache for this allocation if at all possible.
    BlockCache::cached_block_allocation(raw_block)?;

    // Update the cache with the updated block.
    BlockCache::add_or_update_item(CachedBlock::from_raw(raw_block))?;

    // We don't need to write, since the cache will do it for us.
    Ok(())
}

fn go_update_cached_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
    // Update like windows, but better idk this joke sucks lmao

    // No block allocations, since this is an update.

    // Update the cache with the updated block.
    BlockCache::add_or_update_item(CachedBlock::from_raw(raw_block))?;

    // We don't need to write, since the cache will do it for us on flush.
    Ok(())
}

/// Forcibly writes a block to disk immediately, bypasses the cache.
fn go_force_write_block(raw_block: &RawBlock) -> Result<(), FloppyDriveError> {
    // Load in the disk to write to, ensuring that the header is up to date.

    let mut disk: StandardDisk = super::cache_implementation::disk_load_header_invalidation(raw_block.block_origin.disk)?;

    disk.unchecked_write_block(raw_block)?;
    Ok(())
}