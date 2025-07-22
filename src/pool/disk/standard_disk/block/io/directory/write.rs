// Write a new directory into a directory block

use log::debug;

use crate::pool::{self, disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::{directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem}, inode::{self, inode_struct::{Inode, InodeDirectory, InodeFlags, InodeTimestamp}}, io::directory::types::NamedItem}, standard_disk_struct::StandardDisk}}, pool_actions::pool_struct::{Pool, GLOBAL_POOL}};

impl DirectoryBlock {
    /// Add a new item to this block, extending this block if needed.
    /// Updated blocks are written to disk.
    /// 
    /// 
    /// Consumes the DirectoryBlock, since the data may have been updated.
    /// 
    /// May swap disks, will optionally return to a provided disk.
    /// 
    /// Returns nothing.
    pub fn add_item(self, item: DirectoryItem, return_to: Option<u16>) -> Result<(), FloppyDriveError> {
        go_add_item(self, item, return_to)
    }
    /// Creates a new directory block, and adds its location to the input block.
    /// Blocks are created and updated as needed.
    /// 
    /// Requires a disk pointer back to the origin of Directory Block
    /// this was called on. 
    /// 
    /// Consumes the DirectoryBlock, since the data may have been updated.
    /// 
    /// The name of the new directory must be less than 256 characters long.
    /// Attempting to recreate an already existing directory will panic.
    /// 
    /// May swap disks, will optionally return to a provided disk.
    /// 
    /// Returns nothing.
    pub fn make_directory(self, name: String, return_to: Option<u16>) -> Result<(), FloppyDriveError> {
        go_make_directory(self, name, return_to)
    }
}

fn go_make_directory(directory: DirectoryBlock, name: String, return_to: Option<u16>) -> Result<(), FloppyDriveError> {
    debug!("Attempting to create a new directory with name `{name}`...");
    // Check to make sure this block does not already contain the directory we are trying to add.
    // We dont care if listing the directory puts us somewhere else, because we're immediately going to
    // go get a new directory block, which would possibly just swap disks again, and our final update
    // to the original directory block has its origin already specified with block_origin.
    if directory.contains_item(&NamedItem::Directory(name.clone()), None)?.is_some() {
        // We are attempting to create a duplicate item.
        panic!("Attempted to create duplicate directory!")
    }

    // And make sure the name isn't too long.
    assert!(name.len() < 256);

    // Reserve a spot for the new directory
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
    let mut inode_result = Pool::fast_add_inode(inode)?;

    // Now we add this newly created directory to the calling directory.
    // Flags change depending on wether the new directory ended up on this disk.
    // We also may need to update the location to remove the disk information.
    let mut flags: DirectoryFlags = DirectoryFlags::MarkerBit;
    // We also must mark it as a directory, not a normal file.
    flags.insert(DirectoryFlags::IsDirectory);

    if inode_result.disk.expect("Writing an inode always returns what disk it was put on.") == directory.block_origin.disk {
        // New inode is on the same disk as we started on.
        flags.insert(DirectoryFlags::OnThisDisk);
        // Remove the disk information from the inode location
        inode_result.disk = None;
    }

    // Put it all together
    let final_directory_item = DirectoryItem {
        flags,
        name_length: name.len() as u8,
        name,
        location: inode_result,
    };

    // Put it into the caller directory!
    // We dont need to pass in a return disk, since we will return ourselves next if needed.
    directory.add_item(final_directory_item, None)?;

    // Go back to the return disk if needed
    if let Some(number) = return_to {
        let _ = FloppyDrive::open(number)?;
    };

    // All done!
    Ok(())
}

/// Allocates space for and writes a new directory block.
/// 
/// Returns where the new block is.
/// 
/// May swap disks, does not return to original disk.
fn go_make_new_directory_block() -> Result<DiskPointer, FloppyDriveError> {
    // Ask the pool for a new block
    let get_block = Pool::find_free_pool_blocks(1)?;
    let new_directory_location = get_block.first().expect("1 = 1");

    // Open the new block and write that bastard
    let mut new_blocks_disk: StandardDisk = match FloppyDrive::open(new_directory_location.disk)? {
        pool::disk::drive_struct::DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("Why did asking for a free block return a non standard disk?")
    };

    let new_directory_block: RawBlock = DirectoryBlock::new().to_block(new_directory_location.block);
    new_blocks_disk.checked_write(&new_directory_block)?;

    // All done!
    Ok(*new_directory_location)
}


// Add an item to a directory
fn go_add_item(directory: DirectoryBlock, item: DirectoryItem, return_to: Option<u16>) -> Result<(), FloppyDriveError> {
    debug!("Adding new item to directory...");
    // Persistent vars
    // We may load in other blocks, so these may change
    let mut current_directory: DirectoryBlock = directory.clone();
    let mut block_origin: DiskPointer = directory.block_origin;
    let original_location: DiskPointer = directory.block_origin;
    // If we swap disks, we need to update the item to not be on the local disk anymore.
    let mut item_to_add: DirectoryItem = item;
    
    // Now for the loop
    loop {
        // Try adding the item to the current block
        if current_directory.try_add_item(&item_to_add).is_ok() {
            // Cool! We found a spot!
            break
        }
        // There was not enough room in that block, we need to find the next one.
        block_origin = go_find_next_or_extend_block(current_directory, block_origin)?;
        
        // If we moved to a new disk, we need to update the item
        if original_location.disk != block_origin.disk {
            // New disk
            item_to_add.location.disk = Some(block_origin.disk);
            // Only the marker bit, since we're no longer on the disk we started with.
            item_to_add.flags = DirectoryFlags::MarkerBit
        }
        
        // Load the new directory
        let disk_for_loading = match FloppyDrive::open(block_origin.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => panic!("How are we reading directory info from a non-standard disk?")
        };
        current_directory = DirectoryBlock::from_block(&disk_for_loading.checked_read(block_origin.block)?);
        
        // Time to try again!
        continue;
    }
    
    // Now that the loop has ended, we need to write the block that we just updated.
    let mut disk = match FloppyDrive::open(block_origin.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("How are we writing directory info to a non-standard disk?")
    };
    
    // We assume the block has already been reserved, we are simply updating it.
    let to_write: RawBlock = current_directory.to_block(block_origin.block);
    disk.checked_update(&to_write)?;
    
    // Go to a disk if the caller wants.
    if let Some(number) = return_to {
        let _ = FloppyDrive::open(number)?;
    }
    
    debug!("Item added.");
    // Done!
    Ok(())
}

/// Finds the next section of this directory, or extends it if there is none.
///
/// May swap disks, will return to original disk.
fn go_find_next_or_extend_block(directory: DirectoryBlock, block_origin: DiskPointer) -> Result<DiskPointer, FloppyDriveError> {
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
    let mut updated_directory = directory;
    updated_directory.next_block = block_to_load;

    // Write that back.
    let mut disk: StandardDisk = match FloppyDrive::open(block_origin.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("How did we get a non-standard disk?"),
    };

    disk.checked_update(&updated_directory.to_block(block_origin.block))?;

    // All done.
    Ok(block_to_load)
}