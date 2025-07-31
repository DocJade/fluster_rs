// Write a new directory into a directory block

use log::{debug, trace};

use crate::pool::{
    disk::{
        drive_struct::{FloppyDrive, FloppyDriveError, JustDiskType},
        generic::{
            block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO,
        },
        standard_disk::{
            block::{
                directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem},
                inode::inode_struct::{Inode, InodeDirectory, InodeFlags, InodeTimestamp},
                io::directory::types::NamedItem,
            },
        },
    },
    pool_actions::pool_struct::Pool,
};

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
    pub fn add_item(
        self,
        item: &DirectoryItem,
        return_to: Option<u16>,
    ) -> Result<(), FloppyDriveError> {
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
    pub fn make_directory(
        self,
        name: String,
        return_to: Option<u16>,
    ) -> Result<(), FloppyDriveError> {
        go_make_directory(self, name, return_to)
    }
}

fn go_make_directory(
    directory: DirectoryBlock,
    name: String,
    return_to: Option<u16>,
) -> Result<(), FloppyDriveError> {
    debug!("Attempting to create a new directory with name `{name}`...");
    // Check to make sure this block does not already contain the directory we are trying to add.
    // We dont care if listing the directory puts us somewhere else, because we're immediately going to
    // go get a new directory block, which would possibly just swap disks again, and our final update
    // to the original directory block has its origin already specified with block_origin.
    if directory
        .find_item(&NamedItem::Directory(name.clone()), None)?
        .is_some()
    {
        // We are attempting to create a duplicate item.
        panic!("Attempted to create duplicate directory!")
    }

    // And make sure the name isn't too long.
    assert!(name.len() < 256);

    // Reserve a spot for the new directory
    trace!("Getting a new directory block...");
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
    trace!("Adding the inode for the new directory...");
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
    let final_directory_item = DirectoryItem {
        flags,
        name_length: name.len() as u8,
        name,
        location: inode_result,
    };

    // Put it into the caller directory!
    // We dont need to pass in a return disk, since we will return ourselves next if needed.
    trace!("Adding the new directory to the caller...");
    directory.add_item(&final_directory_item, None)?;

    // Go back to the return disk if needed
    if let Some(number) = return_to {
        let _ = FloppyDrive::open(number)?;
    };

    // All done!
    debug!("Done creating directory.");
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
    let new_directory_block: RawBlock = DirectoryBlock::new(*new_directory_location).to_block();

    CachedBlockIO::write_block(&new_directory_block, new_directory_location.disk, JustDiskType::Standard)?;

    // All done!
    Ok(*new_directory_location)
}

// Add an item to a directory
fn go_add_item(
    directory: DirectoryBlock,
    item: &DirectoryItem,
    return_to: Option<u16>,
) -> Result<(), FloppyDriveError> {
    debug!("Adding new item to directory...");

    // Added items must have their flag set.
    assert!(item.flags.contains(DirectoryFlags::MarkerBit));

    // Persistent vars
    // We may load in other blocks, so these may change
    let mut current_directory: DirectoryBlock = directory.clone();
    let mut new_block_origin: DiskPointer = directory.block_origin;
    let original_location: DiskPointer = directory.block_origin;
    // If we swap disks, we need to update the item to not be on the local disk anymore.
    // We clone here so higher up we can keep directory items that are added to directories instead of consuming them on write.
    let mut item_to_add: DirectoryItem = item.clone(); 

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
        current_directory = DirectoryBlock::from_block(&read_block);

        // Time to try again!
        continue;
    }

    // Now that the loop has ended, we need to write the block that we just updated.
    // We assume the block has already been reserved, we are simply updating it.
    let to_write: RawBlock = current_directory.to_block();
    CachedBlockIO::update_block(&to_write, to_write.block_origin.disk, JustDiskType::Standard)?;

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
fn go_find_next_or_extend_block(
    directory: DirectoryBlock,
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
    let mut updated_directory = directory;
    updated_directory.next_block = block_to_load;

    let raw_block: RawBlock = updated_directory.to_block();
    CachedBlockIO::update_block(&raw_block, raw_block.block_origin.disk, JustDiskType::Standard)?;

    // All done.
    Ok(block_to_load)
}
