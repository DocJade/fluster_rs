// Write a new directory into a directory block

use log::{debug, error};

use crate::pool::{
    disk::{
        drive_struct::{FloppyDriveError, JustDiskType},
        generic::{
            block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO,
        },
        standard_disk::block::{
                directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem},
                inode::inode_struct::{Inode, InodeBlock, InodeDirectory, InodeFlags, InodeTimestamp},
                io::directory::types::NamedItem,
            },
    },
    pool_actions::pool_struct::Pool,
};

impl DirectoryBlock {
    /// Add a new item to this block, extending this block if needed.
    /// Updated blocks are written to disk.
    ///
    /// Updates the block that was passed in, since the contents of the block may have changed.
    ///
    /// Returns nothing.
    pub fn add_item(
        &mut self,
        item: &DirectoryItem,
    ) -> Result<(), FloppyDriveError> {
        go_add_item(self, item)
    }
    /// Creates a new directory block, and adds its location to the input block.
    /// Blocks are created and updated as needed.
    ///
    /// Updates the directory block that was passed in.
    ///
    /// The name of the new directory must be less than 256 characters long.
    /// Attempting to recreate an already existing directory will panic.
    ///
    /// Returns the created directory as a DirectoryItem.
    pub fn make_directory(
        &mut self,
        name: String,
    ) -> Result<DirectoryItem, FloppyDriveError> {
        go_make_directory(self, name)
    }

    /// Remove a the given directory. Removes all blocks that contained information about this directory, and updates
    /// all other blocks to remove references to this directory.
    /// 
    /// You must also pass in the DirectoryItem that refers to this directory. You should extract it from the parent
    /// directory.
    /// 
    /// The directory block must be empty of all items.
    /// 
    /// Consumes the incoming block, since it will no longer exist.
    /// 
    /// May swap disks.
    /// 
    /// Returns nothing on success.
    pub fn delete_self(self, self_item: DirectoryItem) -> Result<(), FloppyDriveError> {
        // In theory, as long as the caller used an extracted directory item to call
        // this method, even if this call fails, all references to it will now be gone on
        // a directory level. So even if the inode or the block wasn't freed, its still
        // "deleted", but just leaked its blocks. Which is unfortunate, but fine.


        // Make sure the directory is empty.
        // Caller must check.
        if !self.list()?.is_empty() {
            panic!("Cannot delete an non-empty directory!");
        }

        // Directories should shrink when items are removed. An empty
        // directory should only be 1 block in size.
        // Thus, we only have to deallocate ourselves.
        
        // Remove our inode.
        // We need to find it manually, since we will be updating the
        // inode block.
        let read: RawBlock = CachedBlockIO::read_block(self_item.location.to_disk_pointer(), JustDiskType::Standard)?;
        let mut inode_block: InodeBlock = InodeBlock::from_block(&read);

        if let Err(error) = inode_block.try_remove_inode(self_item.location.offset) {
            // Not good. Something was wrong with the inode pointer.
            // This is a very very very bad thing.
            // The inode blocks may be corrupted.
            // We cannot recover.
            panic!("Tried to remove an invalid inode. Unrecoverable. {error:#?}")
        }

        // Write back the updated inode block
        CachedBlockIO::update_block(&inode_block.to_block(), JustDiskType::Standard)?;

        // Now we can free the block that the directory occupied.
        let freed = Pool::free_pool_block_from_disk(&[self.block_origin])?;
        // This should obviously be one.
        assert_eq!(freed, 1);

        // All done, directory deleted.
        drop(self); // So long
        drop(self_item); // Space Cowboy
        Ok(())
    }
}

fn go_make_directory(
    directory: &mut DirectoryBlock,
    name: String,
) -> Result<DirectoryItem, FloppyDriveError> {
    debug!("Attempting to create a new directory with name `{name}`...");
    // Check to make sure this block does not already contain the directory we are trying to add.
    // We dont care if listing the directory puts us somewhere else, because we're immediately going to
    // go get a new directory block, which would possibly just swap disks again, and our final update
    // to the original directory block has its origin already specified with block_origin.
    debug!("Checking if a directory with that name already exists...");
    if directory
    .find_item(&NamedItem::Directory(name.clone()))?
    .is_some()
    {
        // We are attempting to create a duplicate item.
        error!("ATTEMPTED TO CREATE A DUPLICATE DIRECTORY! PANICKING!");
        panic!("Attempted to create duplicate directory!")
    }
    
    debug!("Name is free.");

    // And make sure the name isn't too long.
    assert!(name.len() < 256);

    // Reserve a spot for the new directory
    debug!("Getting a new directory block...");
    let new_directory_location = go_make_new_directory_block()?;

    // Now that we've made the directory, we need an inode that points to it.

    // Since this is a brand new directory, this inode will have a creation and modified time of right now
    let now = InodeTimestamp::now();

    let inode: Inode = Inode {
        flags: InodeFlags::MarkerBit, // No file bit, since this is a directory
        file: None,
        directory: Some(InodeDirectory::from_disk_pointer(new_directory_location)),
        created: now,
        modified: now,
    };

    // Go put it somewhere.
    debug!("Adding the inode for the new directory...");
    let mut inode_result = Pool::fast_add_inode(inode)?;

    // Now we add this newly created directory to the calling directory.
    // Flags change depending on wether the new directory ended up on this disk.
    // We also may need to update the location to remove the disk information.
    let mut flags: DirectoryFlags = DirectoryFlags::MarkerBit;
    // We also must mark it as a directory, not a normal file.
    flags.insert(DirectoryFlags::IsDirectory);

    let disk_it_ended_up_on = inode_result
        .disk
        .expect("Writing an inode always returns what disk it was put on.");

    if disk_it_ended_up_on == directory.block_origin.disk {
        // New inode is on the same disk as we started on.
        flags.insert(DirectoryFlags::OnThisDisk);
        // Remove the disk information from the inode location
        inode_result.disk = None;
    } else {
        // New inode is on a different disk
        flags.remove(DirectoryFlags::OnThisDisk);
        // Set what disk its from.
        inode_result.disk = Some(disk_it_ended_up_on);
    }

    // Put it all together
    let mut final_directory_item = DirectoryItem {
        flags,
        name_length: name.len() as u8,
        name,
        location: inode_result,
    };

    // Put it into the caller directory!
    // We dont need to pass in a return disk, since we will return ourselves next if needed.
    debug!("Adding the new directory to the caller...");
    directory.add_item(&final_directory_item)?;

    // Now that we've added it to the directory block, since we are returning the directory item again, we need
    // to put the disk number back if we just removed it, since new item that comes out of this function needs
    // to act just like a freshly read item.

    final_directory_item.location.disk = Some(disk_it_ended_up_on);

    // All done!
    debug!("Done creating directory.");
    Ok(final_directory_item)
}

/// Allocates space for and writes a new directory block.
///
/// Returns where the new block is.
///
/// May swap disks, does not return to original disk.
fn go_make_new_directory_block() -> Result<DiskPointer, FloppyDriveError> {
    // Ask the pool for a new block
    // No crc, will overwrite.
    let get_block = Pool::find_and_allocate_pool_blocks(1, false)?;
    let new_directory_location = get_block.first().expect("1 = 1");

    // Open the new block and write that bastard
    let new_directory_block: RawBlock = DirectoryBlock::new(*new_directory_location).to_block();

    CachedBlockIO::update_block(&new_directory_block,JustDiskType::Standard)?;

    // All done!
    Ok(*new_directory_location)
}

// Add an item to a directory
fn go_add_item(
    directory: &mut DirectoryBlock,
    item: &DirectoryItem,
) -> Result<(), FloppyDriveError> {
    debug!("Adding new item to directory...");

    // Added items must have their flag set.
    assert!(item.flags.contains(DirectoryFlags::MarkerBit));

    // Persistent vars
    // We may load in other blocks, so these may change
    let original_location: DiskPointer = directory.block_origin;
    let mut new_block_origin: DiskPointer;
    let mut current_directory: &mut DirectoryBlock = directory;
    // If we swap disks, we need to update the item to not be on the local disk anymore.
    // We clone here so higher up we can keep directory items that are added to directories instead of consuming them on write.
    let mut item_to_add: DirectoryItem = item.clone();

    // Need to hold this out here or the borrow will be dropped.
    let mut next_directory: DirectoryBlock;

    // Now for the loop
    loop {
        // Try adding the item to the current block
        if current_directory.try_add_item(&item_to_add).is_ok() {
            // Cool! We found a spot!
            break;
        }
        // There was not enough room in that block, we need to find the next one.
        new_block_origin = go_find_next_or_extend_block(current_directory)?;

        // If we moved to a new disk, we need to update the item if it was local.
        if original_location.disk != new_block_origin.disk {
            // New disk
            // We only change if it was local to begin with.
            if item_to_add.location.disk.is_none() {
                item_to_add.location.disk = Some(new_block_origin.disk);
                // Remove the flag for being on this disk
                // We still need to preserve the other bits, so we use remove.
                item_to_add.flags.remove(DirectoryFlags::OnThisDisk);
            }
            // Otherwise we would be removing information about where that inode points to, so
            // we wont touch it unless it hasn't been set.
        }

        // Load the new directory
        let read_block: RawBlock = CachedBlockIO::read_block(new_block_origin, JustDiskType::Standard)?;
        next_directory = DirectoryBlock::from_block(&read_block);
        current_directory = &mut next_directory;

        // Time to try again!
        continue;
    }

    // Now that the loop has ended, we need to write the block that we just updated.
    // We assume the block has already been reserved, we are simply updating it.
    let to_write: RawBlock = current_directory.to_block();
    CachedBlockIO::update_block(&to_write, JustDiskType::Standard)?;

    debug!("Item added.");
    // Done!
    Ok(())
}

/// Finds the next section of this directory, or extends it if there is none.
/// 
/// Needs a mutable reference, since the pointer may change.
///
/// May swap disks, will return to original disk.
fn go_find_next_or_extend_block(
    directory: &mut DirectoryBlock,
) -> Result<DiskPointer, FloppyDriveError> {
    let mut block_to_load: DiskPointer = directory.next_block;

    // Make sure we actually have somewhere to go.
    if !directory.next_block.no_destination() {
        // Already have another block to go to.
        return Ok(block_to_load);
    }

    // Looks like we need a new block
    // Get the block in question.
    block_to_load = go_make_new_directory_block()?;

    // Now we must update the previous block to point to this new one.
    directory.next_block = block_to_load;

    let raw_block: RawBlock = directory.to_block();
    CachedBlockIO::update_block(&raw_block, JustDiskType::Standard)?;

    // All done.
    Ok(block_to_load)
}
