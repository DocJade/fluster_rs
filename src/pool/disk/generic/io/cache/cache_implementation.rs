// Non-public cache construction

// Some details about the cache:
// The lowest tier, 0, is completely emptied when it's full. Since we
//  assume that the data within there is of very low quality. If it was
//  worth keeping around, it would have been promoted already
// Tier 1 pushes it's best cached item to tier 2 when it's full.
// Tier 2 discards its least valuable cache item when it's full.
// Within tiers, items are promoted to a higher position whenever a read
//  successfully hits them. The only exception to this is tier 0, where
//  successful reads promote an item up to tier 1.

// When a new item is added to a tier, it starts in the highest position, as it
//  is the most fresh. It is expected that if this item is weaker than pre-existing
//  items, that the newly added top item will quickly slide down in rank.

// The lower cache tiers are inherently more volatile, so they need to be
//  larger to support more opportunities for items to promote before being
//  thrashed out of the cache. Thus we will split the cache into:
// 0: 1/2   of total allowed cache size
// 1: 1/4th of total allowed cache size
// 2: 1/4th of total allowed cache size
// It may seem weird to make the highest tier the same size as the one below it,
//  but items that reach this tier are now such a high quality that they would be
//  very quickly replaced if they became stale, since the constant read hits that are
//  expected of these items would move stale items to the lowest positions very quickly.

// Promotion within tiers always moves the item from whatever index it's currently at, to
//  the very top of the tier. This should ensure that the hottest items stay close to the
//  top, previously I used bubble sort, which could lead to slightly less used items to
//  not promote away from the bottom of the queue fast enough.

use std::{
    collections::{
        HashMap,
        VecDeque
    },
    sync::Mutex
};

use lazy_static::lazy_static;
use log::debug;

use crate::{
    error_types::drive::DriveIOError,
    pool::disk::{
        drive_struct::{
            DiskType,
            FloppyDrive,
        },
        generic::{
            block::{allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock
            },
            disk_trait::GenericDiskMethods,
            generic_structs::pointer_struct::DiskPointer,
            io::{
                cache::{
                    cache_io::CachedBlockIO,
                    cached_allocation::CachedAllocationDisk,
                    statistics::BlockCacheStatistics
                },
                checked_io::CheckedIO
            }
        },
        standard_disk::standard_disk_struct::StandardDisk
    }
};

//
// =========
// GLOBAL? LOCAL? IDK
// =========
//

// The maximum amount of blocks all caches can store
const CACHE_SIZE: usize = 2880 * 2;

// The actual cached data
lazy_static! {
    static ref CASHEW: Mutex<BlockCache> = Mutex::new(BlockCache::new());
}

//
// =========
// STRUCTS
// =========
//

/// The wrapper around all the cache tiers
/// Only avalible within the cache folder,
/// all public interfaces are built on top of CachedBlockIO.
pub(super) struct BlockCache {
    // The different levels of cache.
    // All of the internals are private.

    /// Highest quality, items in this level came from the highest spot from the tier below when
    /// it was completely full. IE filled with the best of level_1.
    tier_2: TieredCache,
    /// Might be useful, promoted from level 0 after being read at least once.
    tier_1: TieredCache,
    /// Unproven items, might as well be garbage.
    tier_0: TieredCache,
}

/// The actual caches
#[derive(Clone)]
struct TieredCache {
    /// How big this cache is.
    size: usize,
    /// The items currently in the cache, hashmap pair
    items_map: HashMap<DiskPointer, CachedBlock>,
    /// Keep track of the order of items in the cache
    order: VecDeque<DiskPointer>
}

/// The cached blocks
/// Available in the cache folder to provide conversion methods.
#[derive(Debug, Clone)]
pub(super) struct CachedBlock {
    /// Where this block came from.
    block_origin: DiskPointer,
    /// The content of the block.
    data: Vec<u8>,
    /// Wether or not this block needs to be flushed.
    /// 
    /// Blocks that are read but never written do not need to be flushed.
    requires_flush: bool
}

//
// =========
// Implementations
// =========
//

// The entire cache
// These functions are public to the cache folder, since we need these for read/write
impl BlockCache {
    /// Create a new empty cache
    fn new() -> Self {
        // Get the max size of the cache
        let size: usize = CACHE_SIZE;
        // Need the 3 tiers
        // Division rounds down, so this is fine.
        let tier_0: TieredCache = TieredCache::new(size/2);
        let tier_1: TieredCache = TieredCache::new(size/4);
        let tier_2: TieredCache = TieredCache::new(size/4);
        // All done
        Self {
            tier_0,
            tier_1,
            tier_2,
        }
    }

    /// Retrieves an item from the cache if it exists.
    /// 
    /// Updates the underlying caches to promote the read item.
    pub(super) fn try_find(pointer: DiskPointer) -> Option<CachedBlock> {
        go_try_find_cache(pointer)
    }

    /// Add an item to the cache, or update it if the item is already present.
    /// 
    /// If the item is new, it will be placed in the lowest tier in the cache.
    pub(super) fn add_or_update_item(item: CachedBlock) -> Result<(), DriveIOError> {
        go_add_or_update_item_cache(item)
    }

    /// get the hit-rate of the cache
    pub(super) fn get_hit_rate() -> f32 {
        BlockCacheStatistics::get_hit_rate()
    }

    // Promotes a tier 0 cache item upwards.
    fn promote_item(&mut self, item: CachedBlock) {
        go_promote_item_cache(self, item)
    }

    /// Removes an item from the cache if it exists.
    /// 
    /// You must flush this item to disk yourself (if needed), or you will lose data!
    /// 
    /// Returns nothing.
    pub(super) fn remove_item(pointer: &DiskPointer) {
        go_remove_item_cache(pointer)
    }

    /// Reserve a block on a disk, skipping the disk if possible.
    /// 
    /// Panics if block was already allocated.
    pub(super) fn cached_block_allocation(raw_block: &RawBlock) -> Result<(), DriveIOError> {
        let mut cache_disk: CachedAllocationDisk = CachedAllocationDisk::open(raw_block.block_origin.disk)?;
        let _ = cache_disk.allocate_blocks(&vec![raw_block.block_origin.block])?;
        // Shouldn't even need to check if it allocated one block, no way it could allocate more.
        Ok(())
    }
    
    /// Flushes all information in a tier to disk.
    /// 
    /// Caller must drop all references to cache before calling this.
    pub(super) fn flush(tier_number: usize) -> Result<(), DriveIOError> {
        go_flush_tier(tier_number)
    }

    /// Drops items from this cache tier that have not been updated, and thus don't need to be written to disk.
    /// 
    /// You should really only call this on tier 0, since items in the higher tiers are usually very read heavy, thus
    /// are usually not updated. Cleaning up those higher tiers would almost certainly discard valuable blocks.
    /// 
    /// Caller must drop all references to cache before calling this.
    /// 
    /// Returns how many blocks were discarded, or None if the tier was already empty.
    pub(super) fn cleanup_tier(tier_number: usize) -> Option<u64> {
        go_cleanup_tier(tier_number)
    }
}

// Cache tiers
impl TieredCache {
    /// Create a new, empty tier of a set size
    fn new(size: usize) -> Self {
        go_make_new_tier(size)
    }
    /// Check if an item is in this tier.
    /// 
    /// Adds a hit to the tier statistics if found, otherwise
    /// leaves the statistics alone.
    /// 
    /// Returns the index of the item if it exists.
    /// 
    /// Does not update tier order.
    fn find_item(&self, pointer: &DiskPointer) -> Option<usize> {
        go_find_tier_item(self, pointer)
    }
    /// Retrieves an item from this tier at the given index.
    /// 
    /// Will promote the item within this tier.
    /// 
    /// Updates tier order.
    /// 
    /// Returns None if there is no item at the index.
    fn get_item(&mut self, index: usize) -> Option<&CachedBlock> {
        go_get_tier_item(self, index)
    }
    /// Extracts an item at an index, removing it from the tier.
    /// 
    /// Returns None if there is no item at the index.
    fn extract_item(&mut self, index: usize) -> Option<CachedBlock> {
        go_extract_tier_item(self, index)
    }
    /// Adds an item to this tier. Will be the new highest item in the tier.
    /// 
    /// Will panic if tier is already full.
    fn add_item(&mut self, item: CachedBlock) {
        go_add_tier_item(self, item)
    }
    /// Updates / replaces an item at a given index.
    /// 
    /// Updates order.
    /// 
    /// Will panic if index is empty / out of bounds.
    fn update_item(&mut self, index: usize, new_item: CachedBlock) {
        go_update_tier_item(self, index, new_item)
    }
    /// Pops the best item of the tier.
    /// 
    /// Returns None if the tier is empty
    fn get_best(&mut self) -> Option<CachedBlock> {
        go_get_tier_best(self)
    }
    /// Pops the worst item of the tier.
    /// 
    /// Returns None if the tier is empty
    fn get_worst(&mut self) -> Option<CachedBlock> {
        go_get_tier_worst(self)
    }
    /// Check if this tier is full
    fn is_full(&self) -> bool {
        go_check_tier_full(self)
    }
}

// Nice to haves for the CachedBlocks
impl CachedBlock {
    /// Turn a CachedBlock into a RawBlock
    pub(super) fn into_raw(self) -> RawBlock {
        RawBlock {
            block_origin: self.block_origin,
            data: self.data.try_into().expect("Should be 512 bytes."),
        }
    }
    /// Turn a RawBlock into a CachedBlock
    /// 
    /// Expects the raw block to already have a disk set.
    pub(super) fn from_raw(block: &RawBlock, requires_flush: bool) -> Self {
        Self {
            block_origin: block.block_origin,
            data: block.data.to_vec(),
            requires_flush
        }
    }
}

//
// =========
// BlockCache Functions
// =========
//

fn go_try_find_cache(pointer: DiskPointer) -> Option<CachedBlock> {

    // Make sure this is a valid disk pointer, otherwise something is horribly wrong.
    assert!(!pointer.no_destination());

    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let cache = &mut CASHEW.try_lock().expect("Single threaded.");

    // Try from highest to lowest
    // Tier 2
    if let Some(found) = cache.tier_2.find_item(&pointer) {
        // In the highest rank!
        BlockCacheStatistics::record_hit();
        // Grab it, which will also update the order.
        return cache.tier_2.get_item(found).cloned()
    }

    // Tier 1
    if let Some(found) = cache.tier_1.find_item(&pointer) {
        // Somewhat common it seems.
        BlockCacheStatistics::record_hit();
        // Grab it, which will also update the order.
        return cache.tier_1.get_item(found).cloned()
    }

    // Tier 0
    if let Some(found) = cache.tier_0.find_item(&pointer) {
        // Scraping the barrel, but at least it was there!
        BlockCacheStatistics::record_hit();
        // Since this is the lowest tier, we need to immediately promote this
        let item = cache.tier_0.extract_item(found).expect("Just checked.");
        cache.promote_item(item.clone());

        // Promotion done, return the item we got.
        return Some(item)
    }

    // It wasn't in the cache. Record the miss.
    BlockCacheStatistics::record_miss();

    // All done.
    None
}

fn go_promote_item_cache(cache: &mut BlockCache, t0_item: CachedBlock) {
    // This is where the magic happens.

    // Since tiers only change size or have new items added to them when tier 0 has a good read,
    // we only have to implement a cache-wide promotion scheme for tier 0.

    // See if there is room in tier 1
    if !cache.tier_1.is_full() {
        // There was room.
        cache.tier_1.add_item(t0_item);
        return
    }

    // There was not room, we need to move an item upwards.
    let t1_best: CachedBlock = cache.tier_1.get_best().expect("How are we empty and full?");

    if !cache.tier_2.is_full() {
        // not full, directly add it.
        cache.tier_2.add_item(t1_best);
    } else {
        // The best cache is full.
        // We will have to move the worst tier 2 item to tier 0. If we discarded it
        // outright, the block it contains would never get flushed to disk.
        let worst_of_2 = cache.tier_2.get_worst().expect("How are we empty and full?");

        // Since we popped an item from t0 to call this function, it must now have at least
        // one slot open, so we can add to it.
        cache.tier_0.add_item(worst_of_2);


        // Now put that tier 1 item in tier 2 to make room for the new tier 1 item from tier 0.
        // Confused yet?
        cache.tier_2.add_item(t1_best);
    }

    // Now that tier 1 has had room made, add the t0 to t1
    cache.tier_1.add_item(t0_item);

    // All done!
}

fn go_add_or_update_item_cache(block: CachedBlock) -> Result<(), DriveIOError> {

    // Make sure the block has a valid location
    assert!(!block.block_origin.no_destination());

    // We don't update the cache statistics in here, since a hit while updating makes no sense.

    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let mut cache = CASHEW.try_lock().expect("Single threaded.");

    // Since we search for the item in every tier before adding, this prevents duplicates.

    // Top to bottom.

    if let Some(index) = cache.tier_2.find_item(&block.block_origin) {
        // Fancy block!
        cache.tier_2.update_item(index, block);
        return Ok(())
    }

    if let Some(index) = cache.tier_1.find_item(&block.block_origin) {
        // Useful!
        cache.tier_1.update_item(index, block);
        return Ok(())
    }

    // Annoyingly, we still have to update the garbage, since reading presumes that stuff in tier 0 is up to date.

    if let Some(index) = cache.tier_0.find_item(&block.block_origin) {
        // Polished garbage.
        cache.tier_0.update_item(index, block);
        return Ok(())
    }

    // It wasn't in any of the tiers, so we will add it to tier 0.
    
    // Make sure we have room first
    if cache.tier_0.is_full() {
        // We don't have room, so we need to flush out tier 0 of the cache.
        // But first we can try dropping items that do not require flushing
        drop(cache);
        if BlockCache::cleanup_tier(0).is_none() {
            // Nothing was removed from the tier, so we have to flush normally, since tier
            // zero is full of items we must write.
            BlockCache::flush(0)?;
        }


        let cache: &mut std::sync::MutexGuard<'_, BlockCache> = &mut CASHEW.try_lock().expect("Single threaded.");
        cache.tier_0.add_item(block);
        return Ok(());
    }

    // Put it in
    cache.tier_0.add_item(block);
    Ok(())
}

fn go_remove_item_cache(pointer: &DiskPointer) {
    // If we just find and extract on every tier, that works
    // Slow? Maybe...
    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let cache = &mut CASHEW.try_lock().expect("Single threaded.");

    // Since we are clearing just one item, not a whole disk, we only need to check each tier once, since there
    // cant be any duplicates, and we can return as soon as we see a matching item.

    if let Some(index) = cache.tier_2.find_item(pointer) {
        // We discard the removed item. We assume the caller already
        // grabbed their own copy if they needed it.
        let _ = cache.tier_2.extract_item(index);
        return
    }

    if let Some(index) = cache.tier_1.find_item(pointer) {
        let _ = cache.tier_1.extract_item(index);
        return
    }

    if let Some(index) = cache.tier_0.find_item(pointer) {
        let _ = cache.tier_0.extract_item(index);
    }

}

//
// =========
// TieredCache Functions
// =========
//


fn go_make_new_tier(size: usize) -> TieredCache {
    // New tiers are obviously empty.
    let mut new_hashmap: HashMap<DiskPointer, CachedBlock> = HashMap::with_capacity(size);
    new_hashmap.shrink_to(size);
    let mut new_order: VecDeque<DiskPointer> = VecDeque::new();
    new_order.reserve_exact(size);
    TieredCache {
        size,
        items_map: new_hashmap,
        order: new_order
    }
}

fn go_find_tier_item(tier: &TieredCache, pointer: &DiskPointer) -> Option<usize> {
    // Does not update order
    // Just see if it exists.

    // Skip if the tier is empty
    if tier.order.is_empty() {
        return None;
    }

    // We check the order, because we care about index here, not the actual block.
    tier.order.iter().position(|x| x == pointer)
}

fn go_get_tier_item(tier: &mut TieredCache, index: usize) -> Option<&CachedBlock> {
    // Updates order

    // Find what item the index refers to
    let wanted_block_pointer: DiskPointer = tier.order.remove(index)?;

    // Now get that item
    let the_block = tier.items_map.get(&wanted_block_pointer)?;

    // Now move the item to the front of the tier, since we have read it
    tier.order.push_front(wanted_block_pointer);

    Some(the_block)
}

fn go_extract_tier_item(tier: &mut TieredCache, index: usize) -> Option<CachedBlock> {
    // Pops an item from any index, preserves order of other items

    // Find the item
    let wanted_block_pointer: DiskPointer = tier.order.remove(index)?;

    // Go get it
    tier.items_map.remove(&wanted_block_pointer)
}

fn go_add_tier_item(tier: &mut TieredCache, item: CachedBlock) {
    // New tier items go at the front, since they are the freshest.
    assert!(!tier.is_full());

    // Put the pointer into the ordering
    tier.order.push_front(item.block_origin);

    // Add to the hashmap
    let already_existed = tier.items_map.insert(item.block_origin, item);

    // Make sure that did not already exist
    assert!(already_existed.is_none());
}

fn go_update_tier_item(tier: &mut TieredCache, index: usize, mut new_item: CachedBlock) {
    // Replace the item, IE the contents of the block have changed.

    // Updating is an access after all... so we will promote it.

    // Update the order
    let to_move = tier.order.remove(index).expect("Should exist.");
    tier.order.push_front(to_move);

    // Set the flush boolean, since this block has been updated with new content, it is
    // out of sync with the disk
    new_item.requires_flush = true;

    // Now replace the item in the hashmap at the index.
    let replaced = tier.items_map.insert(to_move, new_item);
    
    // Make sure we actually replaced it. Not adding here!
    assert!(replaced.is_some());
}

fn go_get_tier_best(tier: &mut TieredCache) -> Option<CachedBlock> {
    // Best is at the front

    // Get the pointer
    let front_pointer = tier.order.pop_front()?;

    // Get the block
    // This will return an option, its the callers fault if this item does not exist.
    tier.items_map.remove(&front_pointer)
}

fn go_get_tier_worst(tier: &mut TieredCache) -> Option<CachedBlock> {
    // The worst item is at the end of the vec
    
    // Get the pointer
    let front_pointer = tier.order.pop_back()?;

    // Get the block
    // This will return an option, its the callers fault if this item does not exist.
    tier.items_map.remove(&front_pointer)
}

fn go_flush_tier(tier_number: usize) -> Result<(), DriveIOError> {
    debug!("Flushing tier {tier_number} of the cache...");
    // We will be flushing all data from this tier of the cache to disk.
    // This can be used on any tier, but will usually be called on tier 0.
    
    // We will extract all of the cache items at once, leaving the tier empty.
    let items_map_to_flush: HashMap<DiskPointer, CachedBlock>;
    let items_order_to_flush: VecDeque<DiskPointer>;
    // We only get the order just to discard it.
    
    // Keep the cache locked within just this area.
    {
        // Get the block cache
        let mut cache = CASHEW.try_lock().expect("Single threaded.");
        
        // find the tier we need to flush
        let tier_to_flush: &mut TieredCache = match tier_number {
            0 => &mut cache.tier_0,
            1 => &mut cache.tier_1,
            2 => &mut cache.tier_2,
            _ => panic!("Bro there are only 3 cache tiers"),
        };
        
        // If the tier is empty, there's nothing to do.
        if tier_to_flush.order.is_empty() {
            return Ok(());
        }
        
        // Move all items from the tier into our local variable,
        // leaving the cache's tier empty.
        
        // In theory, if the flush fails, we would now lose data...
        // just dont fail lol, good luck
        
        items_map_to_flush = std::mem::take(&mut tier_to_flush.items_map);
        items_order_to_flush = std::mem::take(&mut tier_to_flush.order);
    }
    
    let _ = items_order_to_flush;
    
    // Cache is now unlocked
    
    // first we grab all of the items and sort them by disk, low to high, and also sort the blocks
    // within those disks to be in order. Since if the blocks are in order, the head doesn't have to move around
    // the disk as much.
    
    // Get the items from the hashmap
    let mut items: Vec<CachedBlock> = items_map_to_flush.into_values().collect();

    // Before sorting, we can toss any blocks that do not have flush set, since
    // they were never updated and thus don't need to be written back to disk.
    items.retain(|block| block.requires_flush);

    // If we ended up with no items, that means the tier was completely filled with items
    // that did not need to be flushed, and we can exit early.
    if items.is_empty() {
        // Cool
        return Ok(());
    }

    // There are still items in here, we have work to do.

    // Sort the blocks we will actually be writing to put the same disks in order, then by block order.
    items.sort_unstable_by_key(|item| (item.block_origin.disk, item.block_origin.block));
    
    // Now to reduce head movement even further, we don't want to check the allocation table
    // while making our writes. Since that would require seeking to block 0 after each write.
    
    // You might be thinking, "Why can't we use the cache for the allocation tables?", darn good idea,
    // but we cannot access the cache from down here, since that would require locking the entire cache
    // a second time. Also we might be out of room in the cache for the read required to get the table,
    // which would cause us to flush the tier again, which we are already doing. Bad news.
    
    // But there are some assumptions we can make about the items we are flushing:
    // - We assume the items within the cache are valid. (A given, but can't hurt to mention)
    // - If an item is contained within a cache tier, the block it came from must
    //    be allocated, and moreover, unchanged since the last time we flushed to it.
    // - We currently have full control over the floppy disk. Since all high-level
    //    IO happens on the cache itself, we can swap disks and even finish on a
    //    completely different disk without worrying about other callers.
    // - - Furthermore, since we have full control over the disk, the allocation tables
    //      cannot be changing.
    // - When an item is removed from the cache manually, it must have been flushed to disk.
    // - Invalidated items on cache levels higher than 0 will put their invalidated item into
    //    tier zero, thus they will be flushed to disk when it is cleared.
    
    // Basically, we don't have to care about the allocation table AT ALL down here. If
    // we have a block, we know it is allocated. When a block is freed, it must be removed
    // from the cache entirely.
    
    // Therefore, we can make all of our writes in one pass per disk, and never have to look at
    // the allocation table at all!
    
    // To properly allow lazy-loading disks into the drive, we allow the disk loading routine to use cached blocks
    // if they exist.
    
    // The problem is, this causes the disk check to always return true if the header is in the cache, meaning
    // in theory, an incorrect disk can be in the drive.
    
    // To solve this, down here we must grab the header from the cache if it is there, then 
    // we hold onto that, load the disk (which now has to do a proper block read to check if its the right disk), then
    // update the disk if its the correct one.

    // This is the only place that actual disk writes ever happen in normal operation outside of disk initialization.
    
    // Open the first disk to write to
    
    
    // Now we can chunk together the blocks into larger continuous writes for speed.
    // First chunk by disk
    let chunked_by_disk = items.chunk_by(|a,b| a.block_origin.disk == b.block_origin.disk);
    
    // Now we can loop over the disks
    for disk_chunk in chunked_by_disk {
        // open the disk
        let mut current_disk: StandardDisk = disk_load_header_invalidation(disk_chunk[0].block_origin.disk)?;

        // Now chunk together the blocks.
        // Comparison adds instead of subtracts to prevent overflow.
        let chunked_by_block = disk_chunk.chunk_by(|a, b| b.block_origin.block == a.block_origin.block + 1);

        // Now loop over those.
        for block_chunk in chunked_by_block {
            // If this chunk only has one item in it, do a normal write.
            if block_chunk.len() == 1 {
                current_disk.checked_update(&block_chunk[0].clone().into_raw())?;
                continue;
            }

            // There are multiple blocks in a row to update, we need to stitch their bytes together.
            let bytes_to_write: Vec<u8> = block_chunk.iter().flat_map(|block| block.data.clone()).collect();

            // Now do the large write.
            // Unchecked since the headers for the disk may still be in the cache.
            current_disk.unchecked_write_large(bytes_to_write, block_chunk[0].block_origin)?;
        }

    }
    
    // All done, don't need to do any cleanup for previously stated reasons
    debug!("Done flushing tier {tier_number} of the cache.");
    
    Ok(())
}

// Returns an option on if any blocks were freed, and how many.
fn go_cleanup_tier(tier_number: usize) -> Option<u64> {
    // Discard all items in this tier that don't need to be written back to disk.
    debug!("Cleaning up tier {tier_number} of the cache...");

    // Usually I would scope the cache, but we'll be doing these operations without touching the disk.

    // Get the block cache
    let mut cache = CASHEW.try_lock().expect("Single threaded.");
    
    // find the tier we need to flush
    let tier_to_flush: &mut TieredCache = match tier_number {
        0 => &mut cache.tier_0,
        1 => &mut cache.tier_1,
        2 => &mut cache.tier_2,
        _ => panic!("Bro there are only 3 cache tiers"),
    };
    
    // If the tier is empty, there's nothing to do.
    if tier_to_flush.order.is_empty() {
        return None;
    }

    // Now go through all the tier items and check if we can discard them.

    let mut blocks_discarded: u64 = 0;
    
    let blocks_to_cleanup_map = &mut tier_to_flush.items_map;
    let blocks_to_cleanup_order = &mut tier_to_flush.order;

    // To be clever, we can use retain, and only retain the items that do need to be written, otherwise discarding
    // the blocks we dont need as we come across them.
    blocks_to_cleanup_order.retain(|pointer| {
        // Get the block from the hashmap
        let block = blocks_to_cleanup_map.get(pointer).expect("If there's a key, there's a block.");
        if block.requires_flush {
            // This needs to be flushed, so we return true to hold onto this block.
            return true; // Weird that return works in here, never seen that before.
        }
        // Block does not need to be flushed! Discard it.
        let _ = blocks_to_cleanup_map.remove(pointer);

        // Increment the discard count
        blocks_discarded += 1;

        // Return false to discard this pointer from the order vec
        false
    });

    // Unneeded blocks have now been discarded.
    
    // If we weren't able to free anything, we still need to return None here.
    if blocks_discarded == 0 {
        debug!("All blocks in tier require flushing to disk.");
        return None;
    }
    
    debug!("Dropped {blocks_discarded} un-needed blocks from the tier.");
    Some(blocks_discarded)
}

fn go_check_tier_full(tier: &TieredCache) -> bool {
    tier.order.len() == tier.size
}

/// Function for handling the possibility of cached disk headers.
/// This can only be used in the cache.
/// 
/// This should be used in place of direct disk opening to ensure headers are up to date.
pub(in super::super::cache) fn disk_load_header_invalidation(disk_number: u16) -> Result<StandardDisk, DriveIOError> {
    // Try to find the header for this disk in the cache

    let header_pointer: DiskPointer = DiskPointer {
        disk: disk_number,
        block: 0,
    };

    let possibly_cached: Option<RawBlock>;
    if let Some(cached) = CachedBlockIO::try_read(header_pointer) {
        // block is in the cache, hold onto it
        possibly_cached = Some(cached);
        // And remove it from the cache
        CachedBlockIO::remove_block(&header_pointer);
    } else {
        // it isnt there
        possibly_cached = None;
    }

    // Now we can load in the disk without worrying about the header being cached already.
    

    #[allow(deprecated)] // This is being used for the cache.
    let mut disk: DiskType = match FloppyDrive::open(disk_number)? {
        // Due to the caching nature of writing headers, blank disks dont
        // immediately get their header written, even though they are now
        // standard disks, this also applies to unknown disks.
        // Thus, unless its a pool disk, it gets let through.
        // // Maybe, commented out others to see...
        DiskType::Standard(standard_disk) => DiskType::Standard(standard_disk),
        // DiskType::Blank(blank) => DiskType::Blank(blank),
        // DiskType::Unknown(unknown) => DiskType::Unknown(unknown),
        _ => unreachable!("Cache cannot be used for pool disks."),
    };

    // Update the header on the disk if needed.
    if let Some(cached_block) = possibly_cached {
        // There was a header in the cache, so we now need to update the disk again
        disk.checked_update(&cached_block)?;

        // Now the disk is out of sync, we need to load it in _again_
        #[allow(deprecated)] // This is being used for the cache.
        let some_disk = FloppyDrive::open(disk_number)?;
        disk = match some_disk {
            DiskType::Standard(standard_disk) => DiskType::Standard(standard_disk),
            // We dont put blank here again, since the header must be written at this point.
            _ => unreachable!("Cache cannot be used for non-standard disks."),
        };
    }

    // At this point, this must be a standard disk.
    let standard_disk: StandardDisk = disk.try_into().expect("Must be standard at this point");

    // The header on the disk is now up to date.
    Ok(standard_disk)
}