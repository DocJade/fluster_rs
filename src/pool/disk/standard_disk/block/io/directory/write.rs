// Write a new directory into a directory block

use crate::pool::{self, disk::{drive_struct::{FloppyDrive, FloppyDriveError}, generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::{directory::directory_struct::{DirectoryBlock, DirectoryItem}, inode::{self, inode_struct::{Inode, InodeDirectory, InodeFlags, InodeTimestamp}}}, standard_disk_struct::StandardDisk}}, pool_struct::{Pool, GLOBAL_POOL}};

impl DirectoryBlock {
    /// Add a new item to this block
    /// Returns where the new item ended up.
    fn add_item(&mut self, item: DirectoryItem) -> Result<DiskPointer, FloppyDriveError> {
        todo!()
    }
    /// Creates a new directory block, and adds it location to the input block.
    /// Modifies the DirectoryBlock this was called on, but does not write it to disk.
    /// 
    /// The name of the new directory must be less than 256 characters long.
    /// Attempting to recreate an already existing directory will panic.
    /// 
    /// Returns nothing.
    fn make_directory(&mut self, name: String) -> Result<(), FloppyDriveError> {
        go_make_directory(self, name)
    }
}

fn go_make_directory(block: &mut DirectoryBlock, name: String, current_disk: u16) -> Result<(), FloppyDriveError> {
    // Check to make sure this block does not already contain the directory we are trying to add
    if block.contains_item(name) {
        // We are attempting to create a duplicate item.
        panic!("Attempted to create duplicate directory!")
    }

    // And make sure the name isn't too long.
    assert!(name.len() < 256);

    // To create a new directory, first we need a block for the start of the new directory. So request one.
    // We need to grab the pool for this
    let pool = GLOBAL_POOL.get().expect("Single threaded").lock().expect("Single threaded");

    // We dont need to keep track of where we are, since we are just updating the directory block, not writing it back to the disk.
    // It is the callers responsibility to write the changed block back to disk.

    // Get a free block
    let get_block = pool.find_free_pool_blocks(1)?;
    let new_directory_location = get_block.first().expect("1 = 1");

    // Done using the global pool
    drop(pool);

    // Create the new directory block at that location
    // Swap to the new disk if needed.
    let mut new_blocks_disk: StandardDisk = match FloppyDrive::open(new_directory_location.disk)? {
        pool::disk::drive_struct::DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("Why did asking for a free block return a non standard disk?")
    };

    // Now make a new directory block
    let new_directory_block: RawBlock = DirectoryBlock::new().to_block(new_directory_location.block);

    // TODO: In the far future, if anyone still cares, everything form this point onwards should really 
    // be done in a transactional way, where if part of this operation fails, we revert all the changes made.
    // But it is a meme filesystem after all, so...

    // Write that bastard
    new_blocks_disk.checked_write(&new_directory_block)?;

    // Now that we've made the directory, we need an inode that points to it.

    // Since this is a brand new directory, this inode will have a creation and modified time of right now
    let now = InodeTimestamp::now();

    let inode: Inode = Inode {
        flags: InodeFlags::MarkerBit, // No file bit, since this is a directory
        file: None,
        directory: Some(InodeDirectory::from_disk_pointer(*new_directory_location)),
        created: now,
        modified: now,
    };

    // Go put it somewhere
    let inode_result = Pool::fast_add_inode(inode)?;

    // Now we add this newly created directory to the calling directory.
    block.add_item(
        item: DirectoryItem {
            flags: todo!(),
            name_length: name.len(),
            name,
            location: todo!(),
        };
    );
    

    todo!()
}