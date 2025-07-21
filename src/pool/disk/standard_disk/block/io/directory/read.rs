// Higher level abstractions for reading directories.

use log::debug;

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::io::checked_io::CheckedIO, standard_disk::block::directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem}};

// Need a way to search for either a file or a directory
#[derive(Ord, PartialEq, Eq, PartialOrd)]
pub enum NamedItem {
    File(String),
    Directory(String)
}

impl NamedItem {
    /// Extracts the type's name, and the name of that type. (ie "file", "test.txt")
    pub fn debug_strings(&self) -> (&'static str, &String) {
        match self {
            NamedItem::File(name) => ("file", name),
            NamedItem::Directory(name) => ("directory", name),
        }
    }
}

impl DirectoryBlock {
    /// Check if this directory contains an item with the provided name and type.
    /// Returns Option<DirectoryItem> if it exists.
    /// 
    /// May swap disks.
    /// 
    /// Optionally returns to a specified disk after checking.
    pub fn contains_item(&self, item_to_find: &NamedItem, disk_to_return_to: Option<u16>) -> Result<Option<DirectoryItem>, FloppyDriveError> {
        let extracted_debug = item_to_find.debug_strings();
        debug!("Checking if a directory contains the {} `{}`...", extracted_debug.0, extracted_debug.1);
        // Get items
        let items = self.list(disk_to_return_to)?;

        // Look for the provided type
        let named_items: Vec<NamedItem> = items
            .iter()
            .map(|item| {
                if item.flags.contains(DirectoryFlags::IsDirectory) {
                    NamedItem::Directory(item.name.clone()) // TODO: Possibly change file calls to return &str
                } else {
                    NamedItem::File(item.name.clone())
                }
            }).collect();

        // Look for the requested item in the new vec, the index into this vec will be the same
        // as the index into the og items vec
        if let Ok(index) = named_items.binary_search(item_to_find) {
            // It's in there!
            return Ok(Some(items[index].clone()));
        } else {
            // The item wasn't in there.
            return Ok(None);
        }
    }
    /// Returns an Vec of all items in this directory ordered by their String's sort order.
    /// 
    /// May swap disks.
    /// 
    /// Optionally returns to a specified disk after gathering directory items.
    pub fn list(&self, disk_to_return_to: Option<u16>) -> Result<Vec<DirectoryItem>, FloppyDriveError> {
        go_list_directory(self, disk_to_return_to)
    }
}

// Functions

fn go_list_directory(block: &DirectoryBlock, disk_to_return_to: Option<u16>) -> Result<Vec<DirectoryItem>, FloppyDriveError> {
    debug!("Listing a directory...");
    // We need to iterate over the entire directory and get every single item.
    // We assume we are handed the first directory in the chain.
    let mut items_found: Vec<DirectoryItem> = Vec::new();
    let mut current_dir_block: DirectoryBlock = block.clone();

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Add all of the contents of the current directory to the total
        items_found.extend_from_slice(&current_dir_block.get_items());

        // I want to get off Mr. Bone's wild ride
        if current_dir_block.next_block.no_destination() {
            // We're done!
            break
        }

        // Time to load in the next block.
        let disk = match FloppyDrive::open(current_dir_block.next_block.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Why did the block point to a non-standard disk?"),
        };

        current_dir_block = DirectoryBlock::from_block(&disk.checked_read(current_dir_block.next_block.block)?);

        // Onwards!
        continue;
    }
    
    // Sort all of the items by name, not sure what internal order it is, but it will be
    // sorted by whatever comparison function String uses.
    items_found.sort_unstable_by(|a,b| a.name.cmp(&b.name));

    // Return to the specified disk if needed.
    if let Some(number) = disk_to_return_to {
        // Gotta go.
        // We don't care about the resulting disk, just that
        // its in the drive.
        _ = FloppyDrive::open(number)?;
    }

    Ok(items_found)
}