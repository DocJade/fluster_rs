// Higher level abstractions for reading directories.

use log::{debug, trace};

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::io::checked_io::CheckedIO, standard_disk::block::{directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem}, io::directory::types::NamedItem}};

impl DirectoryBlock {
    /// Check if this directory contains an item with the provided name and type.
    /// This checks the entire directory, not just the current block.
    /// Returns Option<DirectoryItem> if it exists.
    /// You must specify which disk this block came from.
    /// 
    /// May swap disks.
    /// 
    /// Optionally returns to a specified disk when done.
    pub fn contains_item(&self, item_to_find: &NamedItem, return_to: Option<u16>) -> Result<Option<DirectoryItem>, FloppyDriveError> {
        let extracted_debug = item_to_find.debug_strings();
        debug!("Checking if a directory contains the {} `{}`...", extracted_debug.0, extracted_debug.1);
        // Get items
        let items: Vec<DirectoryItem> = self.list(return_to)?;

        // Look for the requested item in the new vec, the index into this vec will be the same
        // as the index into the og items vec
        if let Some(item) = item_to_find.find_in(&items) {
            // It's in there!
            debug!("Yes it did.");
            return Ok(Some(item));
        } else {
            // The item wasn't in there.
            debug!("No it didn't.");
            return Ok(None);
        }
    }
    /// Returns an Vec of all items in this directory ordered alphabetically descending.
    /// 
    /// Returned DirectoryItem(s) will have their InodeLocation's disk set.
    /// 
    /// May swap disks.
    /// 
    /// Optionally returns to a specified disk after gathering directory items.
    pub fn list(&self, return_to: Option<u16>) -> Result<Vec<DirectoryItem>, FloppyDriveError> {
        go_list_directory(self, return_to)
    }
}

// Functions

fn go_list_directory(block: &DirectoryBlock, return_to: Option<u16>) -> Result<Vec<DirectoryItem>, FloppyDriveError> {
    debug!("Listing a directory...");
    // We need to iterate over the entire directory and get every single item.
    // We assume we are handed the first directory in the chain.
    let mut items_found: Vec<DirectoryItem> = Vec::new();
    let mut current_dir_block: DirectoryBlock = block.clone();
    // To keep track of what disk an inode is from
    let mut current_disk: u16 = block.block_origin.disk;

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Add all of the contents of the current directory to the total
        // But we will add the disk location data to these structs, it is the responsibility of the caller
        // to remove these disk locations if they no longer need them.
        // Otherwise if we didn't add the disk location for every item, it would be impossible
        // to know where a local pointer goes.
        let mut new_items = current_dir_block.get_items();
        for item in &mut new_items {
            // If the disk location is already there, we wont do anything.
            if item.location.disk.is_none() {
                // There was no disk information, it must be local.
                item.location.disk = Some(current_disk)
            }
            // Otherwise there was already a disk being pointed to.
            // Overwriting it here would corrupt it.
        };

        items_found.extend_from_slice(&new_items);

        // I want to get off Mr. Bone's wild ride
        if current_dir_block.next_block.no_destination() {
            // We're done!
            trace!("Done getting DirectoryItem(s).");
            break
        }

        trace!("Need to continue on the next block.");
        // Time to load in the next block.
        let next_block = current_dir_block.next_block;

        // Update what disk we're on
        current_disk = next_block.disk;

        let disk = match FloppyDrive::open(next_block.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Why did the block point to a non-standard disk?"),
        };

        current_dir_block = DirectoryBlock::from_block(&disk.checked_read(next_block.block)?);

        // Onwards!
        continue;
    }
    
    // Sort all of the items by name, not sure what internal order it is, but it will be
    // sorted by whatever comparison function String uses.
    items_found.sort_by_key(|item| item.name.to_lowercase());

    // Return to a specified block if the caller requested it
    if let Some(number) = return_to {
        _ = FloppyDrive::open(number)?;
    }

    Ok(items_found)
}