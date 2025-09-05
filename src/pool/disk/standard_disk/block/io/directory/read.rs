// Higher level abstractions for reading directories.

use log::{debug, error, warn};

use crate::{error_types::drive::DriveError, pool::{
    disk::{
        generic::{
            block::block_structs::RawBlock,
            generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO
        },
        standard_disk::block::{
            directory::directory_struct::{
                DirectoryBlock, DirectoryItem, DirectoryItemFlags
            },
            io::directory::types::NamedItem,
        }
    },
    pool_actions::pool_struct::Pool
}, tui::{notify::NotifyTui, tasks::TaskType}};

impl DirectoryBlock {
    /// Check if this directory contains an item with the provided name and type.
    /// This checks the entire directory, not just the current block.
    /// 
    /// Returns Option<DirectoryItem> if it exists.
    ///
    /// May swap disks.
    pub fn find_item(
        &self,
        item_to_find: &NamedItem,
    ) -> Result<Option<DirectoryItem>, DriveError> {
        let extracted_debug = item_to_find.debug_strings();
        debug!(
            "Checking if a directory contains the {} `{}`...",
            extracted_debug.0, extracted_debug.1
        );

        // Special case if we are trying to find the root directory.
        if self.is_root() {
            // This is the root directory, are we trying to find a nameless directory?
            if *item_to_find == NamedItem::Directory("".to_string()) {
                // This was a lookup on the root directory.
                debug!("Caller was looking for the root directory item, skipping lookup...");
                return Ok(Some(Pool::get_root_directory_item()));
            }
        }
        
        // No need to have a task if its the root dir, since that's nearly instant.
        let handle = NotifyTui::start_task(TaskType::FindItemInDirectory(extracted_debug.1.to_string()), 2);
        
        // Get items
        let items: Vec<DirectoryItem> = self.list()?;
        NotifyTui::complete_task_step(&handle);
        
        // Look for the requested item in the new vec, the index into this vec will be the same
        // as the index into the og items vec
        if let Some(item) = item_to_find.find_in(&items) {
            // It's in there!
            NotifyTui::complete_task_step(&handle);
            NotifyTui::finish_task(handle);
            debug!("Yes it did.");
            Ok(Some(item))
        } else {
            // The item wasn't in there.
            NotifyTui::complete_task_step(&handle);
            NotifyTui::finish_task(handle);
            debug!("No it didn't.");
            Ok(None)
        }
    }
    /// Returns an Vec of all items in this directory ordered alphabetically descending.
    ///
    /// Returned DirectoryItem(s) will have their InodeLocation's disk set.
    ///
    /// May swap disks.
    pub fn list(&self) -> Result<Vec<DirectoryItem>, DriveError> {
        go_list_directory(self)
    }

    /// Get the size of a directory by totalling all of the items contained within it.
    /// 
    /// Does not recurse into sub-directories. (Seems to be standard behavior in ls -l)
    /// 
    /// Returns the size in bytes.
    pub fn get_size(&self) -> Result<u64, DriveError> {
        debug!("Getting size of a directory...");
        // get all the items
        debug!("Listing items...");
        let items = self.list()?;
        
        debug!("Totaling up item sizes...");
        let mut total_size: u64 = 0;
        for item in items {
            // Ignore if this is a directory.
            // We don't recurse into the next directory, we only get the size of the items
            // directly contained within this directory.
            if item.flags.contains(DirectoryItemFlags::IsDirectory) {
                continue;
            }
            // Get the size of this file
            let inode = item.get_inode()?;
            let file = inode.extract_file().expect("The inode the directory item points at should be a file.");
            total_size += file.get_size()
        }

        // All done
        debug!("Size obtained. `{total_size}` bytes.");
        Ok(total_size)
    }

    /// Check if this DirectoryBlock is the head of the root directory.
    /// 
    /// This will return false on any other block than the head block.
    fn is_root(&self) -> bool {
        // Lives in a static place.
        static ROOT_BLOCK_LOCATION: DiskPointer = DiskPointer {
            disk: 1,
            block: 2,
        };

        self.block_origin == ROOT_BLOCK_LOCATION
    }

    /// Extracts an item from a directory block, blanking out the space it used to occupy.
    /// 
    /// This looks for the item in the entire directory, not just the block this was called on.
    /// Due to this, we assume this is being called on the head of the DirectoryBlock chain.
    /// 
    /// Automatically flushes changes to disk if required.
    /// 
    /// If you just want to get the item for reading or minor modifications, use find_item()
    /// 
    /// Updates the passed in directory block.
    /// 
    /// Returns nothing if the item did not exist.
    pub(crate) fn find_and_extract_item(&mut self, item_to_find: &NamedItem) -> Result<Option<DirectoryItem>, DriveError> {

        // Go find the item.

        // Nice struct to make dealing with this a bit nicer
        struct ItemFound {
            /// The item
            item: DirectoryItem,
            /// This is set if the removal of that item caused the block to be fully emptied.
            /// 
            /// Thus, if this is set, this block needs to be deallocated, and have the block before it
            /// set to point to this new pointer, which points to the block _after_ the block that the item was found in.
            /// 
            /// Slightly confusing.
            empty_thus_new_pointer: Option<DiskPointer>,
            /// Which block in the list we were contained within, indexed from front to back.
            origin_index: usize,

        }

        // Get the blocks
        let mut blocks: Vec<DirectoryBlock> = get_blocks(self.block_origin)?;

        // Find the item, and deduce what block it's in.
        // Index, the item, maybe pointer to the next block
        let mut find: Option<ItemFound> = None;
        for (index, block) in blocks.iter_mut().enumerate() {
            // Is it in here?
            if let Some(found) = block.block_extract_item(item_to_find)? {
                // Cool!
                find = Some(ItemFound {
                    item: found.0,
                    empty_thus_new_pointer: found.1,
                    origin_index: index,
                });
                break
            }

        };

        // Did we find the item?
        let found = match find {
            Some(ok) => ok,
            None => {
                // Item did not exist.
                return Ok(None);
            },
        };

        // If we didn't get a pointer, there is no required cleanup, since no blocks were emptied.
        let new_pointer = match found.empty_thus_new_pointer {
            Some(ok) => ok,
            None => {
                // No cleanup required!
                return Ok(Some(found.item));
            },
        };

        // We got a pointer, thus a block was emptied.

        // If the block that was emptied was the first one in the list, don't need to do anything.
        // Sure, we could shuffle the head forwards, but adding things to directories searches front to back anyways, so
        // switching the pointers around would be needlessly complicated.
        if found.origin_index == 0 {
            // Cool!
            return Ok(Some(found.item));
        }

        // We have emptied a block in the middle of the chain. We need to update the pointer behind us to point
        // past us.

        // This operation is independent to the block in front of us, so no update is required there.

        // If this was the last block in the chain, this will just point to the no_destination pointer, which just marks the
        // new end of the chain.

        let previous_block = &mut blocks[found.origin_index - 1];

        // Now update that block with the new pointer
        previous_block.next_block = new_pointer;

        // Update it.
        let raw_ed = previous_block.to_block();
        CachedBlockIO::update_block(&raw_ed)?;

        // Now we will free that block that was emptied.

        // Now delete the block that we emptied by freeing it.
        let release_me = blocks[found.origin_index].block_origin;
        let freed = Pool::free_pool_block_from_disk(&[release_me])?;
        // this should ALWAYS be 1
        assert_eq!(freed, 1, "We should always free one block when removing an empty directory block in a chain.");
        
        // All done!
        // Update the incoming block head, in case we changed it.
        // Since we need to own this, we'll just pull it out of the vec.
        // The updated block order does not matter, since we're immediately dropping this afterwards.
        *self = blocks.swap_remove(0);
        Ok(Some(found.item))
    }

    /// Extract an item from this directory block, if it exists.
    /// 
    /// Will flush self to disk if block is updated.
    /// 
    /// If the block is now empty, will also return Some() pointer it's next block, regardless
    /// if that block exists or not (will return a final pointer on the last block).
    /// 
    /// Not a public function, use `find_and_extract_item`.
    fn block_extract_item(&mut self, item_to_find: &NamedItem) -> Result<Option<(DirectoryItem, Option<DiskPointer>)>, DriveError> {
        // Do we have the requested item?
        if let Some(found) = item_to_find.find_in(&self.directory_items) {
            // Found the item!
            // Remove it from ourselves.
            self.try_remove_item(&found).expect("Guard, we already know its in there.");
            // Now flush ourselves to disk
            let raw_block = self.to_block();
            CachedBlockIO::update_block(&raw_block)?;

            // If we are now empty, also return a pointer to the next block
            let maybe_pointer: Option<DiskPointer> = if self.get_items().is_empty() {
                // Yep
                Some(self.next_block)
            } else {
                None
            };

            // Now return the item, and the possible pointer to the next block
            return Ok(Some((found, maybe_pointer)))
        }

        // Not in here.
        Ok(None)
    }

    /// Rename an item in place.
    /// 
    /// Searches entire directory for the item.
    /// 
    /// Assumes that the passed in directory block is the head.
    /// 
    /// Returns true if the item existed and was renamed.
    /// 
    /// Flushes change to disk.
    pub(crate) fn try_rename_item(&mut self, to_rename: &NamedItem, new_name: String) -> Result<bool, DriveError> {

        // Since the size of the item might change (name length change) we cant just update the name directly, we have to
        // extract the item and re-add it.

        // This may move the item across disks, thus if its set to local, we must add the disk number.
        // If the disk number is no longer required after its written down, `add_item` will make it local again,

        // We also take in the directory item instead of the named item, since you shouldn't be holding onto it after this.

        // Make sure the name is valid.
        assert!(new_name.len() <= 255, "Name is too long.");

        // Get the item
        if let Some(mut exists) = self.find_and_extract_item(to_rename)? {
            // Copy it, just in case...
            let copy = exists.clone();
            // Now rename it and put it back
            exists.name_length = new_name.len() as u8;
            exists.name = new_name;
            // If this doesn't work, the item is now gone forever lol, thus
            // we will check the result of this operation and try to put the item back if we can.
            let add_result = self.add_item(&exists);
            if add_result.is_ok() {
                // All good.
                Ok(true)
            } else {
                // Addition failed!
                warn!("Adding item during rename failed.");
                warn!("Attempting to restore non-renamed item...");
                if self.add_item(&copy).is_ok() {
                    // That worked
                    warn!("Old item restored.")
                } else {
                    error!("Failed to restore old item during rename failure! Item has been lost!");
                    // Well shit. Not much we can do.
                    println!("Fluster has just lost your file/folder named `{}`, sorry!", copy.name);
                    // we have to give up.
                    panic!("File lost during rename.");
                }
                // We need to fail tests even if the item was restored.
                if cfg!(test) {
                    panic!("Rename failure. Addition failed.")
                }
                // Now we are... fine? The item is still there, it just 
                // wasn't renamed.
                Err(DriveError::Retry)
            }
        } else {
            // No such item.
            Ok(false)
        }
    }
}

// Functions

fn go_list_directory(
    block: &DirectoryBlock,
) -> Result<Vec<DirectoryItem>, DriveError> {
    let handle = NotifyTui::start_task(TaskType::ListingDirectory, 2);
    debug!("Listing a directory...");
    // We need to iterate over the entire directory and get every single item.
    // We assume we are handed the first directory in the chain.
    
    // Get the blocks
    debug!("Getting blocks...");
    let blocks = get_blocks(block.block_origin)?;
    debug!("This directory is made of {} blocks.", blocks.len());
    NotifyTui::complete_task_step(&handle);
    
    // Get the items out of them
    debug!("Getting items...");
    let mut items_found: Vec<DirectoryItem> = blocks.into_iter().flat_map(move |block| {
        block.get_items()
    }).collect();
    NotifyTui::complete_task_step(&handle);
    
    
    // Sort all of the items by name, not sure what internal order it is, but it will be
    // sorted by whatever comparison function String uses.
    debug!("Sorting...");
    items_found.sort_by_key(|item| item.name.to_lowercase());
    NotifyTui::finish_task(handle);
    
    debug!("Directory listing finished.");
    Ok(items_found)
}


/// Starting on the head block of a DirectoryBlock, return every block in the chain, in order.
/// 
/// Does not take in a directory block, since we would need to consume it.
/// 
/// Includes the head block.
fn get_blocks(start_block_location: DiskPointer) -> Result<Vec<DirectoryBlock>, DriveError> {
    // Needing to consume the incoming block would be stinky. But since cloning is not allowed, and we
    // need to return the head block, we have to go get it ourselves.

    // This must be a valid block
    assert!(!start_block_location.no_destination(), "Provided head directory block does not exist!");
    
    let raw_read: RawBlock = CachedBlockIO::read_block(start_block_location)?;
    let start_block: DirectoryBlock = DirectoryBlock::from_block(&raw_read);

    // We assume we are handed the first directory in the chain.
    // Cannot pre-allocate the vec, since we dont know how many blocks there will be.
    let mut blocks: Vec<DirectoryBlock> = Vec::new();
    let mut current_dir_block: DirectoryBlock = start_block;

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Remember where the next block is
        let next_block: DiskPointer = current_dir_block.next_block;
        // Add the current block to the Vec
        blocks.push(current_dir_block);

        // I want to get off Mr. Bone's wild ride
        if next_block.no_destination() {
            // We're done!
            break;
        }
        
        // Load in the next block.
        let next_block_reader = CachedBlockIO::read_block(next_block)?;
        current_dir_block = DirectoryBlock::from_block(&next_block_reader);

        // Onwards!
        continue;
    }
    Ok(blocks)
}