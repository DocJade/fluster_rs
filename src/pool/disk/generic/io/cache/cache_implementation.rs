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

use std::sync::Mutex;

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
    items: Vec<CachedBlock>
}

/// The cached blocks
/// Available in the cache folder to provide conversion methods.
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
    pub(super) fn add_or_update_item(block: CachedBlock) {
        go_add_or_update_item_cache(block)
    }

    /// get the hit-rate of the cache
    pub(super) fn get_hit_rate() -> f32 {
        BlockCacheStatistics::get_hit_rate()
    }
}

// Cache tiers
impl TieredCache {
    /// Create a new, empty cache of a set size
    fn new(size: usize) -> Self {
        go_make_new_tier(size)
    }
    /// Check if an item is in this cache.
    /// 
    /// Returns the index of the item if it exists.
    /// 
    /// Does not update cache order.
    fn find_item(&self, pointer: &DiskPointer) -> Option<usize> {
        go_find_tier_item(self, pointer)
    }
    /// Retrieves an item from this cache at the given index.
    /// 
    /// Will promote the item within this cache.
    /// 
    /// Updates cache order.
    /// 
    /// Returns None if there is no item at the index.
    fn get_item(&self, index: usize) -> Option<CachedBlock> {
        go_get_tier_item(self, index)
    }
    /// Pops the best item of the cache.
    /// 
    /// Returns None if the cache is empty
    fn get_best(&mut self) -> Option<CachedBlock> {
        go_get_tier_best(self)
    }
    /// Pops the worst item of the cache.
    /// 
    /// Returns None if the cache is empty
    fn get_worst(&mut self) -> Option<CachedBlock> {
        go_get_tier_worst(self)
    }
    /// Completely wipes a cache.
    fn reset(&mut self) {
        go_reset_tier(self)
    }
    /// Check if this cache is full
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
            originating_disk: Some(self.block_origin.disk),
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
    /// Will be none if there was not a disk specified
    fn to_pointer(&self) -> Option<DiskPointer> {
        if self.originating_disk.is_none() {
            // Can't make a pointer.
            return None;
        }
        let point = DiskPointer {
            disk: self.block_index,
            block: self.originating_disk.expect("Guarded."),
        };
        return Some(point);
    }
}

//
// =========
// BlockCache Functions
// =========
//

fn go_try_find_cache(pointer: DiskPointer) -> Option<CachedBlock> {
    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let cache = &mut CASHEW.lock().expect("Single threaded.");
    todo!();
}

fn go_add_or_update_item_cache(block: CachedBlock) {
    // To prevent callers from having to lock the global themselves, we will grab it here ourselves
    // and pass it downwards into any functions that require it.
    let cache = &mut CASHEW.lock().expect("Single threaded.");
    todo!();
}

//
// =========
// TieredCache Functions
// =========
//


fn go_make_new_tier(size: usize) -> TieredCache {
    todo!()
}

fn go_find_tier_item(tier: &TieredCache, pointer: &DiskPointer) -> Option<usize> {
    todo!()
}

fn go_get_tier_item(tier: &TieredCache, index: usize) -> Option<CachedBlock> {
    todo!()
}

fn go_get_tier_best(tier: &mut TieredCache) -> Option<CachedBlock> {
    todo!()
}

fn go_get_tier_worst(tier: &mut TieredCache) -> Option<CachedBlock> {
    todo!()
}

fn go_reset_tier(tier: &mut TieredCache) {
    todo!()
}

fn go_check_tier_full(tier: &TieredCache) -> bool {
    todo!()
}