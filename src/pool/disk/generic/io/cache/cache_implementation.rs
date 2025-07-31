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

use std::{collections::VecDeque, sync::Mutex};

use lazy_static::lazy_static;

//
// =========
// GLOBAL? LOCAL? IDK
// =========
//

// The maximum amount of blocks all caches can store
const CACHE_SIZE: usize = 2880;

// The actual cached data
lazy_static! {
    static ref CASHEW: Mutex<BlockCache> = Mutex::new(BlockCache::new());
}

//
// =========
// STRUCTS
// =========
//

use crate::pool::disk::{drive_struct::JustDiskType, generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer, io::cache::statistics::BlockCacheStatistics}};

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
struct TieredCache {
    /// How big this cache is.
    size: usize,
    /// The items currently in the cache.
    items: VecDeque<CachedBlock>
}

/// The cached blocks
/// Available in the cache folder to provide conversion methods.
#[derive(Debug, Clone)]
pub(super) struct CachedBlock {
    /// Where this block came from.
    block_origin: DiskPointer,
    /// The type of disk this came from.
    disk_type: JustDiskType,
    /// The content of the block.
    data: Vec<u8>,
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
    pub(super) fn add_or_update_item(item: CachedBlock) {
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
    /// Completely wipes a tier.
    fn reset(&mut self) {
        go_reset_tier(self)
    }
    /// Check if this tier is full
    fn is_full(&self) -> bool {
        go_check_tier_full(self)
    }
}

// Nice to haves for the CachedBlocks
impl CachedBlock {
    /// Turn a CachedBlock into a RawBlock
    pub(super) fn to_raw(&self) -> RawBlock {
        RawBlock {
            block_index: self.block_origin.block,
            originating_disk: self.block_origin.disk,
            data: self.data.clone().try_into().expect("Should be 512 bytes."),
        }
    }
    /// Turn a RawBlock into a CachedBlock
    /// 
    /// Expects the raw block to already have a disk set.
    pub(super) fn from_raw(block: &RawBlock, disk_type: JustDiskType) -> Self {
        let pointer = block.to_pointer().expect("Disk should be set before conversion.");
        Self {
            block_origin: pointer,
            disk_type,
            data: block.data.to_vec(),
        }
    }
}

// Easier RawBlock to DiskPointer conversions
impl RawBlock {
    /// Convert this block to a disk pointer.
    fn to_pointer(&self) -> Option<DiskPointer> {
        let point = DiskPointer {
            disk: self.block_index,
            block: self.originating_disk,
        };
        Some(point)
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
    let cache = &mut CASHEW.lock().expect("Single threaded.");

    // Try from highest to lowest
    // Tier 2
    if let Some(found) = cache.tier_2.find_item(&pointer) {
        // In the highest rank!
        // Grab it, which will also update the order.
        return cache.tier_2.get_item(found).cloned()
    }

    // Tier 1
    if let Some(found) = cache.tier_1.find_item(&pointer) {
        // Somewhat common it seems.
        // Grab it, which will also update the order.
        return cache.tier_1.get_item(found).cloned()
    }

    // Tier 0
    if let Some(found) = cache.tier_0.find_item(&pointer) {
        // Scraping the barrel, but at least it was there!
        // Since this is the lowest tier, we need to immediately promote this
        let item = cache.tier_0.extract_item(found).expect("Just checked.");
        cache.promote_item(item.clone());

        // Promotion done, return the item we got.
        return Some(item)
    }

    // It wasn't in the cache. Record the miss.
    BlockCacheStatistics::record_hit(false);

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
        // The best cache is full, discard the worst item to make space.
        let _ = cache.tier_2.get_worst().expect("How are we empty and full?");
        // Now there is room.
        cache.tier_2.add_item(t1_best);
    }

    // Now that tier 1 has had room made, add the t0 to t1
    cache.tier_1.add_item(t0_item);

    // All done!
}

fn go_add_or_update_item_cache(block: CachedBlock) {

    // Make sure the block has a valid location
    assert!(!block.block_origin.no_destination());

    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let cache = &mut CASHEW.lock().expect("Single threaded.");

    // Top to bottom.

    if let Some(index) = cache.tier_2.find_item(&block.block_origin) {
        // Fancy block!
        cache.tier_2.update_item(index, block);
        return
    }

    if let Some(index) = cache.tier_1.find_item(&block.block_origin) {
        // Useful!
        cache.tier_1.update_item(index, block);
        return
    }

    // Annoyingly, we still have to update the garbage, since reading presumes that stuff in tier 0 is up to date.

    if let Some(index) = cache.tier_0.find_item(&block.block_origin) {
        // Polished garbage.
        cache.tier_0.update_item(index, block);
        return
    }

    // It wasn't in any of the tiers, so we will add it to tier 0.
    
    // Make sure we have room first
    if cache.tier_0.is_full() {
        // We don't have room, so we need to wipe the cache.
        cache.tier_0.reset();
    }

    // Put it in
    cache.tier_0.add_item(block);
}

//
// =========
// TieredCache Functions
// =========
//


fn go_make_new_tier(size: usize) -> TieredCache {
    // New tiers are obviously empty.
    let mut new_vec: VecDeque<CachedBlock> = VecDeque::new();
    new_vec.reserve_exact(size);
    TieredCache {
        size,
        items: new_vec,
    }
}

fn go_find_tier_item(tier: &TieredCache, pointer: &DiskPointer) -> Option<usize> {
    // Does not update order
    // Just see if it exists.
    tier.items.iter().position(|x| x.block_origin == *pointer)
}

fn go_get_tier_item(tier: &mut TieredCache, index: usize) -> Option<&CachedBlock> {
    // Updates order
    // First do the swap if needed
    if index == 0 {
        // No need to swap, already at the top.
        return tier.items.get(index)
    }
    
    // Do the swap
    tier.items.swap(index - 1, index);
    // return the item, the index has changed.
    tier.items.get(index - 1)
}

fn go_extract_tier_item(tier: &mut TieredCache, index: usize) -> Option<CachedBlock> {
    // Pops an item from any index, preserves order of other items
    tier.items.remove(index)
}

fn go_add_tier_item(tier: &mut TieredCache, item: CachedBlock) {
    // New tier items go at the front, since they are the freshest.
    assert!(!tier.is_full());
    tier.items.push_front(item);
}

fn go_update_tier_item(tier: &mut TieredCache, index: usize, new_item: CachedBlock) {
    // Replace the item
    tier.items[index] = new_item
}

fn go_get_tier_best(tier: &mut TieredCache) -> Option<CachedBlock> {
    // Best is at the front
    tier.items.pop_front()
}

fn go_get_tier_worst(tier: &mut TieredCache) -> Option<CachedBlock> {
    // The worst item is at the end of the vec
    tier.items.pop_back()
}

fn go_reset_tier(tier: &mut TieredCache) {
    // Completely empties the tier
    tier.items.clear();
}

fn go_check_tier_full(tier: &TieredCache) -> bool {
    tier.items.len() == tier.size
}