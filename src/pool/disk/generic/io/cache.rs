use std::{collections::VecDeque, sync::Mutex};

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError, JustDiskType}, generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}};
use lazy_static::lazy_static;
use log::debug;



// In order to reduce disk swapping, we need to keep track of every read/write operation to be able to cache commonly used blocks.
// To facilitate this, I'm gonna rip out all of the pre-existing IO operations and replace them with a cache that must be interacted with instead.

// These new functions will completely replace checked_io functions, since we can now completely abstract away the disk from callers.

// When you open a disk, disk swapping will only happen if you attempt to read a block, and the cache falls through with the read.

// Cache invalidation will be very simple:
// - Check if a block is in the cache
// - - If it is, we take the index of that cached block and swap it with the block in front of it, like bubble sort.
// - - this will cause more frequently accessed blocks to bubble up to the top of the vec.
// - If the block does not exist, we either add the block to the end of the block cache vec if there is room, or replace the lowest
// - ranked cache item.
// If a block is written to, we will remove it from the cache and move all items upwards to fill the gap.

// Holds the cache
const CACHE_SIZE: usize = 2880;
lazy_static! {
    static ref BLOCK_CACHE: Mutex<Vec<CachedBlock>> = Mutex::new(Vec::with_capacity(CACHE_SIZE)); // One floppy worth of blocks.
    static ref CACHE_STATISTICS: Mutex<BlockCacheStatistics> = Mutex::new(BlockCacheStatistics::new());
}

/// The cached blocks
struct CachedBlock {
    /// Where this block came from
    block_origin: DiskPointer,
    /// The type of disk this came from
    disk_type: JustDiskType,
    /// The content of the block
    data: Vec<u8>,
}

/// Statistic information about the cache
struct BlockCacheStatistics {
    /// Stats for calculating cache hit rates
    hits_and_misses: VecDeque<bool>, // we will track the last 1000 reads
    /// How many disk swaps we've prevented
    swaps_saved: u64
}

// New cache
// We will track the last 1000 disk reads
impl BlockCacheStatistics {
    fn new() -> Self {
        Self {
            hits_and_misses: VecDeque::with_capacity(1000),
            swaps_saved: 0,
        }
    }
    fn get_hit_rate(&self) -> f32 {
        if self.hits_and_misses.is_empty() {
            return 0.0
        }
        // rate is hits / total requests
        let hits = self.hits_and_misses.iter().filter(|&&hit| hit).count();
        hits as f32 / self.hits_and_misses.len() as f32
    }
    /// Record a cache hit/miss
    fn record_hit(&mut self, hit: bool) {
        // Need to pop the oldest hit if we're out of room.
        if self.hits_and_misses.len() >= 1000 {
            self.hits_and_misses.pop_front();
        }
        self.hits_and_misses.push_back(hit);
    }
}

/// Struct for implementing cache methods on.
/// Holds no information, this is just for calling.
pub struct BlockCache {
   // m tea
}

// Cache methods
impl BlockCache {
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
    pub fn write_block(raw_block: RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
        go_write_cached_block(raw_block, disk_number, expected_disk_type)
    }
    /// Updates pre-existing block on disk, updates cache.
    /// 
    /// You must specify the type of disk the block is being written to, otherwise you cannot guarantee that you
    /// wrote to the correct disk.
    pub fn update_block(raw_block: RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
        go_update_cached_block(raw_block, disk_number, expected_disk_type)
    }
    /// Check if a block is in the cache.
    /// 
    /// Returns the index of the block, if it exists.
    /// 
    /// This function automatically swaps the blocks to move them up in the chain on read.
    fn find_block(block: &DiskPointer, expected_disk_type: &JustDiskType) -> Option<usize> {
        // The most frequently wanted items will be at the front of the Vec, so a linear search is fine.
        todo!()
    }
    /// Updates the info inside of a block. Does not change block order.
    fn update_or_add_block(block_origin: DiskPointer, expected_disk_type: JustDiskType, data: Vec<u8>) {
        // Check if the block is already in the cache
        if let Some(index) = BlockCache::find_block(&block_origin, &expected_disk_type) {
            // Block was already in the cache, we will update it in-place.
            let updated_block = CachedBlock {
                block_origin,
                disk_type: expected_disk_type,
                data,
            };
            debug!("Updating block cache...");
            BLOCK_CACHE.lock().expect("Single thread")[index] = updated_block;
            debug!("Updated.");
            return
        }
        // This block isn't currently in the cache.
        
        // Pop if there isn't room
        let borrowed_cache: &mut Vec<CachedBlock> = &mut BLOCK_CACHE.lock().expect("Single thread");

        if borrowed_cache.len() == CACHE_SIZE {
            // Pop the last item
            let _ = borrowed_cache.pop();
        }

        // Add the new block to the end.
        let new_block: CachedBlock = CachedBlock {
            block_origin,
            disk_type: expected_disk_type,
            data,
        };
        borrowed_cache.push(new_block);
        return
    }
}

// This function also updates the block order after the read.
fn go_read_cached_block(block_location: DiskPointer, expected_disk_type: JustDiskType) -> Result<RawBlock, FloppyDriveError> {
    // Check if the block is in the cache
    if let Some(index) = BlockCache::find_block(&block_location, &expected_disk_type) {
        // It was in the cache! Return the block...
        let cached = &BLOCK_CACHE.lock().expect("Single thread")[index];
        let constructed: RawBlock = RawBlock {
            block_index: cached.block_origin.block,
            originating_disk: Some(cached.block_origin.disk),
            data: cached.data.clone().try_into().expect("This should be 512 bytes."),
        };
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
    BlockCache::update_or_add_block(block_location, expected_disk_type, read_block.data.to_vec());

    // Return the block.
    return Ok(read_block);
}

fn go_write_cached_block(raw_block: RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
    // Writing time!
    let mut disk = FloppyDrive::open(disk_number)?;
    
    // Make sure this is the write one...
    assert_eq!(disk, expected_disk_type);

    // Just in case...
    assert_ne!(disk, JustDiskType::Blank);
    assert_ne!(disk, JustDiskType::Unknown);

    // Write the block.
    disk.checked_write(&raw_block)?;
    Ok(())
}

fn go_update_cached_block(raw_block: RawBlock, disk_number: u16, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
    // Update like windows, but better idk this joke sucks
    let mut disk = FloppyDrive::open(disk_number)?;
    
    // Make sure this is the write one...
    assert_eq!(disk, expected_disk_type);

    // Just in case...
    assert_ne!(disk, JustDiskType::Blank);
    assert_ne!(disk, JustDiskType::Unknown);

    // Write the block.
    disk.checked_update(&raw_block)?;
    Ok(())
}