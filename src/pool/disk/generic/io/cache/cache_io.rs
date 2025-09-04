// External interaction with the block cache

use crate::{
    error_types::drive::DriveError,
    pool::disk::{
        drive_struct::FloppyDrive,
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock
            },
            disk_trait::GenericDiskMethods,
            generic_structs::pointer_struct::DiskPointer,
            io::cache::{
                cache_implementation::{
                    BlockCache,
                    CachedBlock
                },
            cached_allocation::CachedAllocationDisk
        }
    }, standard_disk::standard_disk_struct::StandardDisk
}, tui::notify::NotifyTui};

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
    pub fn forcibly_write_a_block(raw_block: &RawBlock) -> Result<(), DriveError> {
        go_force_write_block(raw_block)
    }

    /// Attempts to read a block from the cache, does not load from disk if not present.
    /// 
    /// Returns the block if present, or None if absent.
    pub fn try_read(block_origin: DiskPointer) -> Option<RawBlock> {
        if let Some(cached) = BlockCache::try_find(block_origin) {
            // Was there!
            // Tell the TUI
            NotifyTui::read_cached();
            return Some(cached.into_raw())
        }
        // Missing.
        None
    }

    
    /// Check if a block is in the cache, and if it is dirty or not.
    /// 
    /// Returns Some(true) if the block is dirty, false if clean, or None if the block is absent.
    pub fn status_of_cached_block(block_origin: DiskPointer) -> Option<bool> {
        if let Some(cached) = BlockCache::try_find(block_origin) {
            return Some(cached.requires_flush)
        }
        // Missing.
        None
    }

    /// Reads in a block from disk, attempts to read it from the cache first.
    /// 
    /// Block must already be allocated on origin disk.
    /// 
    /// Only works on standard disks.
    pub fn read_block(block_origin: DiskPointer) -> Result<RawBlock, DriveError> {
        go_read_cached_block(block_origin)
    }

    /// Writes a block to disk. Adds newly written block to cache.
    /// 
    /// Block must not be allocated on destination disk, will allocate on write.
    /// 
    /// Only works on standard disks.
    pub fn write_block(raw_block: &RawBlock) -> Result<(), DriveError> {
        go_write_cached_block(raw_block)
    }

    /// Updates pre-existing block on disk, updates cache.
    /// 
    /// Block must be already allocated on the destination disk.
    /// 
    /// Only works on standard disks.
    pub fn update_block(raw_block: &RawBlock) -> Result<(), DriveError> {
        go_update_cached_block(raw_block)
    }

    /// Get the hit-rate of the underlying cache
    pub fn get_hit_rate() -> f64 {
        BlockCache::get_hit_rate()
    }

    /// Sometimes you just need to remove a block from the cache, not even set it to zeros.
    /// 
    /// You MUST flush the block you are passing in before calling this function (if needed), or you WILL lose data!
    pub fn remove_block(block_origin: &DiskPointer) {
        BlockCache::remove_item(block_origin)
    }

    /// Flush the entire cache to disk.
    pub fn flush() -> Result<(), DriveError> {
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
fn go_read_cached_block(block_location: DiskPointer) -> Result<RawBlock, DriveError> {
    // Grab the block from the cache if it exists.

    // Block must be allocated.
    // Unless it is a header, which are always allocated.
    // If we check for header allocation, we would try to open the header for the allocation check, to check if the header is allocated,
    // which would recurse and overflow the stack.
    if block_location.block != 0 {
        // This isn't a header.
        let is_allocated = CachedAllocationDisk::open(block_location.disk)?.is_block_allocated(block_location.block);
        assert!(is_allocated);
    }
    
    let disk_in_drive = FloppyDrive::currently_inserted_disk_number();
    
    if let Some(found_block) = BlockCache::try_find(block_location) {
        // It was in the cache! Return the block...

        // Notify the TUI
        NotifyTui::read_cached();

        // If we would've swapped disks, also increment that
        if disk_in_drive != block_location.disk {
            NotifyTui::swap_saved();
        }

        return Ok(found_block.into_raw());
    }

    
    // The block was not in the cache, we need to go get it old-school style.
    // If we are about to swap disks, we will flush tier 0 of the disk.
    if disk_in_drive != block_location.disk {
        // About to swap, do the flush.
        // Dont care how many blocks this flushes.
        let _ = BlockCache::flush_a_disk(disk_in_drive)?;
    };

    // Now that the cache was flushed (if needed), do the read.
    let disk: StandardDisk = super::cache_implementation::disk_load_header_invalidation(block_location.disk)?;

    // We prefer to read at least 96 blocks, if the extra blocks dont fit, we just discard them.
    // If that fails somehow, we will try just a standard single block read as a fallback.
    // We also check to make sure we got something back, otherwise we have to fall back to the other read style.

    // But if we don't have room for 96 blocks, we will read as many as we can fit.
    let tier_free_space = BlockCache::get_tier_space(0);
    let to_read = std::cmp::min(tier_free_space, 96);

    if let Ok(blocks) = &disk.unchecked_read_multiple_blocks(block_location.block, to_read as u16) && !blocks.is_empty() {
        for block in blocks {
            // If the block is already in the cache, we skip adding it, since
            // it may have been updated already.
            // Silent, or we would be randomly promoting blocks.
            if BlockCache::try_find_silent(block.block_origin).is_some() {
                // Skip
                continue;
            }

            // Add it to the cache, since the block doesn't exist yet.
            BlockCache::add_or_update_item(CachedBlock::from_raw(block, false))?;
        }
        // We have to cast back and forth to clone it. Lol.
        let silly: RawBlock = CachedBlock::from_raw(&blocks[0], false).into_raw();
        return Ok(silly)
    }

    // Need to do a singular read.
    // Already checked if it was allocated.
    let read_block = disk.unchecked_read_block(block_location.block)?;
    
    // Add it to the cache.
    // This is a block read from disk, so we do not set the flush flag.
    BlockCache::add_or_update_item(CachedBlock::from_raw(&read_block, false))?;

    // Return the block.
    Ok(read_block)
}

fn go_write_cached_block(raw_block: &RawBlock) -> Result<(), DriveError> {
    // Write a block to the disk, also updating the cache with the block (or adding it if it does not yet exist.)

    // The cache expects the block's destination to be allocated already, so we will allocate it here.
    // We want to use the cache for this allocation if at all possible.
    BlockCache::cached_block_allocation(raw_block)?;

    // Update the cache with the updated block.
    // This is a write, so this will need to be flushed.
    BlockCache::add_or_update_item(CachedBlock::from_raw(raw_block, true))?;

    // We don't need to write, since the cache will do it for us.

    // Notify the TUI
    NotifyTui::write_cached();

    Ok(())
}

fn go_update_cached_block(raw_block: &RawBlock) -> Result<(), DriveError> {
    // Update like windows, but better idk this joke sucks lmao

    // We have to skip the allocation check if we are attempting to update the header, otherwise
    // this will recuse and overflow the stack

    if raw_block.block_origin.block != 0 {
        // This is not a header.
        // Make sure block is currently allocated.
        let is_allocated = CachedAllocationDisk::open(raw_block.block_origin.disk)?.is_block_allocated(raw_block.block_origin.block);
        assert!(is_allocated);
    }

    // Update the cache with the updated block.
    // This is an update, so it must be flushed, since the block has changed.
    BlockCache::add_or_update_item(CachedBlock::from_raw(raw_block, true))?;

    // Notify the TUI
    NotifyTui::write_cached();

    // We don't need to write, since the cache will do it for us on flush.
    Ok(())
}

/// Forcibly writes a block to disk immediately, bypasses the cache.
fn go_force_write_block(raw_block: &RawBlock) -> Result<(), DriveError> {
    // Load in the disk to write to, ensuring that the header is up to date.
    let mut disk: StandardDisk = super::cache_implementation::disk_load_header_invalidation(raw_block.block_origin.disk)?;
    disk.unchecked_write_block(raw_block)
}