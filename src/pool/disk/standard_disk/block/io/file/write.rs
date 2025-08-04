// Writing files.

// We will take in InodeFile(s) instead of Extent related types, since we need info about how big files are so they are easier to extend.
// Creating files is handles on the directory side, since new files just have a name and location.

use std::{cmp::max, u16};

use crate::pool::{
    disk::{
        drive_struct::{
            FloppyDriveError,
            JustDiskType
        },
        generic::{
            block::{
                block_structs::RawBlock,
                crc::add_crc_to_block
            },
            generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO
        },
        standard_disk::{
            block::{
                directory::directory_struct::{
                    DirectoryBlock,
                    DirectoryFlags,
                    DirectoryItem
                },
                file_extents::{
                    file_extents_methods::DATA_BLOCK_OVERHEAD,
                    file_extents_struct::{
                        ExtentFlags,
                        FileExtent,
                        FileExtentBlock
                    }
                },
                inode::inode_struct::{
                    Inode,
                    InodeBlock,
                    InodeFile,
                    InodeFlags,
                    InodeTimestamp
                },
                io::directory::types::NamedItem
            },
        }
    },
    pool_actions::pool_struct::Pool
};

impl InodeFile {
    /// Update the contents of a file starting at the provided seek point.
    /// Will automatically grow file if needed.
    /// 
    /// !! This does not flush information to disk! You must
    /// write back the new size of the file to disk! !!
    /// 
    /// Optionally returns to a provided disk when done.
    /// 
    /// Returns number of bytes written, but also updates the incoming file's size automatically
    fn write(&mut self, bytes: &[u8], seek_point: u64) -> Result<u32, FloppyDriveError> {
       go_write(self, bytes, seek_point)
    }
}

impl DirectoryBlock {
    /// Create a new empty file in this directory with a name.
    /// 
    /// Adds the file to the directory, flushes it to disk.
    /// 
    /// Returns the created file's directory item. Will contain disk info.
    /// 
    /// Should include extension iirc?
    pub fn new_file(self, name: String) -> Result<DirectoryItem, FloppyDriveError> {
        go_make_new_file(self, name)
    }

    /// Deletes an file by deallocating every block the file used to take up, and removing it
    /// from the directory.
    /// 
    /// If you are looking to truncate the file, you need to call 
    /// 
    /// Returns `None` if the file did not exist.
    ///
    /// Panics if fed a directory. Use remove_directory() !
    pub fn delete_file(&mut self, file: NamedItem) -> Result<Option<()>, FloppyDriveError> {
        // We only handle files here
        assert!(file.is_file());
        
        // Extract the item
        let extracted_item: DirectoryItem;
        if let Some(exists) = self.extract_item(&file)? {
            // Item was there
            extracted_item = exists
        } else {
            // Tried to delete a file that does not exist in this directory.
            return Ok(None)
        };

        // Delete it.
        truncate_or_delete_file(&extracted_item, true)?;

        // Since the extraction function already handles pulling out the item from the directory blocks, we are done.
        Ok(Some(()))
    }
}

// We dont want to call read/write on the inodes, we should do it up here so we
// we can automatically update the information on the file, and the directory if needed.
impl DirectoryItem {
    /// Write data to a file.
    ///
    /// Assumptions:
    /// - This is an FILE, not a DIRECTORY.
    /// - The location of this directory item has it's disk set.
    /// - The inode that the item points at does exist, and is valid.
    /// 
    /// Write to a file at a starting offset, and sets `x` bytes after that offset.
    /// 
    /// Does not consume the directory item, since the data lower down was updated, not the
    /// directory item itself.
    /// 
    /// Returns how many bytes were written.
    /// 
    /// Optionally returns to a specified disk.
    pub fn write_file(&self, bytes: &[u8], seek_point: u64) -> Result<u32, FloppyDriveError> {
        // Is this a file?
        if self.flags.contains(DirectoryFlags::IsDirectory) {
            // Uh, no it isn't why did you give me a dir?
            panic!("Tried to read a directory as a file!")
        }

        // Extract out the file
        assert!(self.location.disk.is_some());
        let location = &self.location;

        // Get the inode block
        let the_pointer_in_question: DiskPointer = DiskPointer {
            disk: location.disk.expect("Guarded"),
            block: location.block,
        };

        let read: RawBlock = CachedBlockIO::read_block(the_pointer_in_question, JustDiskType::Standard)?;
        let mut inode_block: InodeBlock = InodeBlock::from_block(&read);

        // Get the actual file
        let inode_with_file = inode_block.try_read_inode(location.offset).expect("Caller guarantee.");
        let mut file = inode_with_file.extract_file().expect("Caller guarantee.");

        // Write to the file
        // This automatically updates the underlying file with the new size.
        let num_bytes_written = file.write(bytes, seek_point)?;

        // Now that the bytes are written, the size of the file may have changed, so we need to flush this new information to disk.

        // Reconstruct the inode
        let mut updated_inode: Inode = inode_with_file;

        // Replace the inner file with the new updated one
        updated_inode.file = Some(file);

        // We also need to update the modify timestamp.
        updated_inode.modified = InodeTimestamp::now();

        // Now update the inode in the block. This also flushes to disk for us.
        inode_block.update_inode(location.offset, updated_inode)?;

        // All done. Return the number of bytes we wrote.
        Ok(num_bytes_written)
    }

    /// Truncates a file. Deallocates every block that used to hold data for this file.
    /// 
    /// No action needs to be taken after this method.
    /// 
    /// Does not delete the origin FileExtent block.
    /// 
    /// Panics if fed a directory.
    pub fn truncate(&self) -> Result<(), FloppyDriveError> {
        // Make sure this is a file
        assert!(!self.flags.contains(DirectoryFlags::IsDirectory));
        truncate_or_delete_file(self, false)
    }
}

fn go_write(inode_file: &mut InodeFile, bytes: &[u8], seek_point: u64) -> Result<u32, FloppyDriveError> {
    // Decompose the file into its pointers
    // No return location, we don't care where this puts us.
    let mut blocks = inode_file.to_pointers()?;

    // get the seek point
    let (block_index, mut byte_index) = InodeFile::byte_finder( seek_point);

    // The byte_finder already skips the flag, so it ends up adding one, we need to subtract that.
    // TODO: This is a bandaid fix. this logic is ugly.
    byte_index -= 1;

    // Make sure we actually have a block at that offset. We cannot start writing from unallocated space.
    assert!(block_index <= blocks.len());
    
    // Now we can calculate where the final byte of this write will end up.
    // Minus 1, since we are writing to the byte we start the seek from
    // IE: if we write 1 byte from out offset, we don't actually move forwards into the next byte.
    let (mut final_block_index, _) = InodeFile::byte_finder(seek_point + bytes.len() as u64 - 1);

    // If final block index is 0, we still need at least 1 block, since block 0 is the first block.
    // Thus we must always add 1.
    final_block_index += 1;

    // Special case, if we have 0 blocks, everything up till now works fine, but we always need at least 1 block to write into.
    // To make my life easier, we will do it here instead of relying on the caller.

    // Now, if our final block index is larger than how many blocks we currently have, that means we need to pre-allocate the new room.
    if final_block_index > blocks.len() || blocks.len() == 0 {
        // We need to get more room.
        // Special case will always allocate at least one block.
        let needed_blocks = max(final_block_index - blocks.len(), 1);

        // This should always be <= the max write size set in 
        // filesystem_methods.rs. Which should ALWAYS be quite far away
        // from u16::MAX, but we check anyways.

        // We hard cap it to u16 block though, just in case.
        assert!(needed_blocks <= u16::MAX.into());

        // Add that many more blocks to this file.
        // Since we know its already less than u16::MAX this cast is fine.
        let new_pointers = expand_file(*inode_file, needed_blocks.try_into().expect("Guarded."))?;

        // The new pointers are already in order for us, and we will add them onto the end of the
        // pointers we grabbed earlier from the file.
        blocks.extend(new_pointers.iter());
    }

    // Now we know we have enough space for this write, let's get started.

    let mut bytes_written: usize = 0;
    // Since the write can start un-aligned, we need to use an offset until its aligned again.
    let mut byte_write_index: u16 = byte_index;

    // Now we will loop through the blocks starting at the current index
    for block in &blocks[block_index..] {
        // are we out of bytes to write?
        if bytes_written == bytes.len() {
            // All done!
            break
        }
        // Update the block
        let written = update_block(*block, &bytes[bytes_written..], byte_write_index)?;
        // After the first write, the offset should be fixed now, since we've either written all of our bytes, in
        // which case we would be done, or we ran out of room in the block, thus the next block's offset would be 0.
        byte_write_index = 0;
        // Update how many bytes we've written
        bytes_written += written;
        // Keep going!
        continue;
    }

    // Done writing bytes!
    // Update the file size with the new bytes we wrote.
    let before = inode_file.get_size();
    inode_file.set_size(before + bytes_written as u64);

    // Return how many bytes we wrote!
    Ok(bytes_written as u32)
}

/// Updates a block with new content, overwriting previous content at an offset.
/// 
/// You can feed in as many bytes as you like, but it will only write as many as it can.
/// 
/// Offset is the first data byte, not the first byte of the block!
/// 
/// Returns number of bytes written.
fn update_block(block: DiskPointer, bytes: &[u8], offset: u16) -> Result<usize, FloppyDriveError> {

    // How much data a block can hold
    let data_capacity = 512 - DATA_BLOCK_OVERHEAD as usize;
    let offset = offset as usize;

    // Check for impossible offsets
    assert!(offset < data_capacity, "Tried to write outside of the capacity of a block.");
    
    // Calculate bytes to write based on REMAINING space.
    let remaining_space = data_capacity - offset;
    let bytes_to_write = std::cmp::min(bytes.len(), remaining_space);
    
    // We also don't support 0 byte writes.
    // Since that would be a failure mode of the caller, in theory could be
    // stuck in an infinite loop type shi.
    // Why panic? It won't if you fix the caller! :D
    assert_ne!(bytes_to_write, 0, "Tried to write 0 bytes to a block!");


    // load the block
    let mut block_copy: RawBlock = CachedBlockIO::read_block(block, JustDiskType::Standard)?;
    
    // Modify that sucker
    // Skip the first byte with the flag
    let start = offset + 1;
    let end = start + bytes_to_write;

    block_copy.data[start..end].copy_from_slice(&bytes[..bytes_to_write]);

    // Update the crc
    add_crc_to_block(&mut block_copy.data);

    // Write that sucker
    CachedBlockIO::update_block(&block_copy, JustDiskType::Standard)?;

    // Return the number of bytes we wrote.
    Ok(bytes_to_write)
}


/// Expands a file by adding `x` new blocks to the extents. Returns disk pointers for the new extents.
/// Updates underlying ExtentBlock(s) for this file.
/// 
/// May swap disks, does not return to any start disk.
fn expand_file(inode_file: InodeFile, blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    // Go grabby some new blocks.
    // These will be already reserved for us.
    // We also need to write the CRC for later.
    let reserved_blocks = Pool::find_and_allocate_pool_blocks(blocks, true)?;

    // Make some extents from that
    let new_extents = pointers_into_extents(&reserved_blocks);

    // Add the extents.
    expanding_add_extents(inode_file, &new_extents)?;

    // Return the pointers to those new extents.
    Ok(reserved_blocks)
}

/// Expands an ExtentBlockBlock.
/// Always extends by one block.
/// 
/// Will swap disks to the location of the new block. Will not return to the disk the caller started on.
/// 
/// Sets the new destination in incoming block.
fn expand_extent_block(block: &mut FileExtentBlock) -> Result<(), FloppyDriveError> {
    // Get a new block from the pool
    let the_finder = Pool::find_free_pool_blocks(1)?;
    let new_block_location = the_finder.last().expect("Asked for 1.");

    // Put the a block there
    let new_block: RawBlock = FileExtentBlock::new(*new_block_location).to_block();

    // Write, since we looked for a free block, didn't reserve it yet.
    CachedBlockIO::write_block(&new_block, JustDiskType::Standard)?;

    // Now update the block we came in here with
    block.next_block = *new_block_location;

    // Updated! All done.
    Ok(())
}

/// Will automatically deduce runs of blocks from a slice of disk pointers, then add those extents to the block,
/// expanding the block if needed.
/// 
/// We assume the incoming pointers are already sorted by disk, block. (1, 1), (1, 2), (2, 1) etc.
/// 
/// Does not check if blocks are already allocated, caller _MUST_ provide marked blocks.
fn expanding_add_extents(file: InodeFile, extents: &[FileExtent]) -> Result<(), FloppyDriveError> {
    // We will reverse the extents vec so we can pop them off the back
    // for easier adding, avoiding an index.
    let mut new_extents: Vec<FileExtent> = extents.to_vec();
    new_extents.reverse();

    // Go get the extent block to add to.
    // We need the final one in the chain.
    let mut current_extent_block: FileExtentBlock;
    let mut current_disk: u16 = u16::MAX;
    
    // Read in the initial block
    let raw_read: RawBlock = CachedBlockIO::read_block(file.pointer, JustDiskType::Standard)?;
    current_extent_block = FileExtentBlock::from_block(&raw_read);

    loop {
        // Is this the final block?
        if !current_extent_block.next_block.no_destination() {
            // No it isn't. We need to load the next block.
            // Get the block.
            let reader_mc_deeder: RawBlock = CachedBlockIO::read_block(current_extent_block.next_block, JustDiskType::Standard)?;
            current_extent_block = FileExtentBlock::from_block(&reader_mc_deeder);
            current_disk = current_extent_block.next_block.disk;
            // Try again.
            continue;
        }

        // This is the final block.
        // Try adding extents
        loop {
            // Make sure we still have a extent
            if new_extents.is_empty() {
                // We're all done adding!
                break
            }

            // Try adding a new extent.

            // But we need to make sure its properly set to local first if need be.
            let mut updated_extent = new_extents.last().expect("Guarded.").clone();
            // We dont pop it off, since if the write fails, we need to put it in the next block instead.

            if updated_extent.disk_number.expect("Should be set above.") == current_disk {
                // This is a local block, update it to match
                updated_extent.disk_number = None;
                updated_extent.flags.insert(ExtentFlags::OnThisDisk);
            }

            let added_result = current_extent_block.add_extent(updated_extent);
            // if that worked, that means we added the extent successfully.
            if added_result.is_ok() {
                // Good! Keep going
                // Pop off the extent since we're done with it.
                let _ = new_extents.pop();
                continue;
            }
            // Otherwise we either ran out of room, or this isn't the last block in the chain.
            // We already checked the latter, so...
            break
        }
        
        // There are two cases that cause us to break out of that loop.
        if new_extents.is_empty() {
            // We ran out of file extents to add. we are done.
            // Flush it
            flush_to_disk(&current_extent_block)?;
            // bye
            break
        }

        // We must've ran out of room.
        // Expand the block please.
        expand_extent_block(&mut current_extent_block)?;
        
        // Now we must write that extended block to disk.
        flush_to_disk(&current_extent_block)?;

        // The block has a new destination now, and has been flushed to disk. There is nothing
        // else left for us to do before just looping again, since the next loop
        // will see the new destination.
        continue;
    }

    // All done adding extents! We've already flushed the blocks as well.
    // There is nothing left to do.
    Ok(())
}

/// Automatically groups incoming pointers into a new vec of file extents.
/// assumes all of the incoming pointers are already sorted.
fn pointers_into_extents(pointers: &[DiskPointer]) -> Vec<FileExtent> {
    // I feel like there is 100% a better way to do this, but i dont know it. so too bad!
    let mut new_extents: Vec<FileExtent> = Vec::new();
    let mut current_extent: FileExtent = FileExtent::new();

    for pointer in pointers {
        if current_extent.disk_number.is_none() {
            // brand new, add disk and start block.
            current_extent.disk_number = Some(pointer.disk);
            current_extent.start_block = pointer.block;
        }
        // Check if we're still on the correct disk
        if current_extent.disk_number == Some(pointer.disk) {
            // Same disk, but in theory this could have skipped blocks.
            if pointer.block == current_extent.start_block + current_extent.length as u16 {
                // This is the next block, we didn't skip anything.
                // I know this nesting is ugly, but its better than one massive if clause
                // Also make sure we can keep extending
                if current_extent.length != u8::MAX {
                    current_extent.length += 1;
                    continue;
                }
            }
        }
        
        // If we are here, either the disk, or the next block is not correct, or we hit the max size.
        // Time for a new extent.

        // push the current extent
        new_extents.push(current_extent);
        // clear the current extent so we can start over.
        current_extent = FileExtent::new();
        // brand new, add disk and start block.
        // use the current pointer to start the new extent
        current_extent.disk_number = Some(pointer.disk);
        current_extent.start_block = pointer.block;
        // New extent will have a new block, so len==1
        current_extent.length += 1;
    }

    // After the loop there might be one final extent, check for that
    if current_extent.disk_number.is_some() {
        // There is an extent, make sure we didn't already add it.
        let final_extent = new_extents.last();

        // If there is no final extent, we only made one and never got to add it.
        if final_extent.is_none() {
            new_extents.push(current_extent);
        } else {
            // There is at least one other extent.
            // Make sure this isn't a duplicate.
            if *final_extent.expect("Guarded") == current_extent {
                // we already added it.
                // do nothing.
            } else {
                // This is a new one!
                new_extents.push(current_extent);
            }
        }
    }
    new_extents
}

/// Create a new file.
fn go_make_new_file(directory_block: DirectoryBlock, name: String) -> Result<DirectoryItem, FloppyDriveError> {
    // Directory blocks already have a method to add a new item to them, so we just need
    // to create that item to add.

    // New files must have a filename that is <= u8::MAX
    assert!(name.len() <= u8::MAX.into());

    // Timestamp for file creation
    let right_now: InodeTimestamp = InodeTimestamp::now();
    
    let in_progress = Pool::find_free_pool_blocks(1)?;
    let reserved_block: DiskPointer = *in_progress.last().expect("Only asked for one block.");
    
    // Now that we have the new block we need a FileExtentBlock to write into it.
    let new_block: FileExtentBlock = FileExtentBlock::new(reserved_block);

    // No need to set the marker bit since this is a file ofc.

    // Now let's write that new block
    let raw: RawBlock = new_block.to_block();
    // Block is not marked as reserved, so this is a write.
    CachedBlockIO::write_block(&raw, JustDiskType::Standard)?;

    // Construct the file that we'll be returning.
    let finished_new_file: InodeFile = InodeFile::new(reserved_block);

    // Now that the block has been written, put that sucker into the directory
    
    // We do need an inode location tho, so get one
    let new_inode: Inode = Inode {
        flags: {
            // We need to set the marker bit and the inode type (file)
            let mut inner = InodeFlags::MarkerBit;
            inner.insert(InodeFlags::FileType);
            inner
        },
        file: Some(finished_new_file),
        directory: None,
        created: right_now,
        modified: right_now,
    };

    let mut new_inode_location = Pool::fast_add_inode(new_inode)?;

    // Wrap it all up in a little bow to put into the directory

    // Now we need to set up the flags for the new directory item
    // Flag
    let mut flags: DirectoryFlags = DirectoryFlags::MarkerBit;

    // If the new inode location is on this disk we must set the location, otherwise clear it.
    let directory_block_origin_disk = directory_block.block_origin.disk;
    if new_inode_location.disk.expect("Should be there") == directory_block_origin_disk {
        // We need to remove the disk info.
        flags.insert(DirectoryFlags::OnThisDisk);
        new_inode_location.disk = None;
    } else {
        // Otherwise, this is on another disk, and we do not need to do anything.
    }


    let mut new_file: DirectoryItem = DirectoryItem {
        flags,
        name_length: name.len().try_into().expect("Already checked name length."),
        name,
        location: new_inode_location,
    };

    directory_block.add_item(&new_file)?;
    // If we're here, that worked. We are all done adding the item to the directory.

    // This function always returns a directory item with the disk set, regardless if it was local to the
    // DirectoryBlock that was passed in.

    if new_file.location.disk.is_none() {
        // It's not set already, so it was local to the DirectoryBlock, so we grab the disk from that
        new_file.location.disk = Some(directory_block_origin_disk)
    }
    // All done.
    // Dont need to swap disks, already did that on the item add.
    Ok(new_file)
}

// Will only truncate if delete is false.
fn truncate_or_delete_file(item: &DirectoryItem, delete: bool) -> Result<(), FloppyDriveError> {
    // Is this a file?
        if item.flags.contains(DirectoryFlags::IsDirectory) {
            // Uh, no it isn't why did you give me a dir?
            panic!("Tried to read a directory as a file!")
        }

        // Extract out the file
        assert!(item.location.disk.is_some());
        let location = &item.location;

        // Get the inode block
        let the_pointer_in_question: DiskPointer = DiskPointer {
            disk: location.disk.expect("Guarded"),
            block: location.block,
        };

        let read: RawBlock = CachedBlockIO::read_block(the_pointer_in_question, JustDiskType::Standard)?;
        let mut inode_block: InodeBlock = InodeBlock::from_block(&read);

        // Get the actual file
        let mut inode_with_file: Inode = inode_block.try_read_inode(location.offset).expect("Caller guarantee.");
        let mut file: InodeFile = inode_with_file.extract_file().expect("Caller guarantee.");

        // Get all of the blocks that the file is stored in.
        let mut used_blocks: Vec<DiskPointer> = file.to_pointers()?;

        // Now we need to get all of the blocks that the extents take up

        let first_extent: DiskPointer = file.pointer;

        let mut current_extent_block: FileExtentBlock = FileExtentBlock::from_block(&CachedBlockIO::read_block(first_extent, JustDiskType::Standard)?);

        // Loop over the extents, adding the blocks until we hit the end
        while !current_extent_block.next_block.no_destination() {
            // Have a destination, add it to the pile.
            used_blocks.push(current_extent_block.next_block);
            // Next
            current_extent_block = FileExtentBlock::from_block(&CachedBlockIO::read_block(first_extent, JustDiskType::Standard)?);
        }

        // Now we will free all of those blocks

        // But not the first extent if we are not deleting.
        if !delete {
            // Remove the first extent block from the list to delete
            // This has to work, the loop HAD to've added it
            let index_of_first = used_blocks.iter().position(|pointer| *pointer == first_extent).expect("Should have first pointer");
            let _ = used_blocks.swap_remove(index_of_first);
        }

        // Sort blocks by disk and block order
        used_blocks.sort_unstable_by_key(|block| (block.disk, block.block));

        // Split into sections based on when the disk changes
        // I feel like i already wrote this but i cant find it. lol
        // But i know i didn't do it this way before! suck it past me!

        let chunked = used_blocks.chunk_by(|a, b| a == b);


        // Now go free all of those blocks.
        // This will zero out the blocks, and remove them from the cache for us.
        for chunk in chunked {
            let freed = Pool::free_pool_block_from_disk(chunk)?;
            assert_eq!(freed as usize, chunk.len());
        }

        // Now all of those blocks have been freed.

        // If we are deleting the file, we dont need to do anything else, since the caller will just discard the directory item.
        if delete {
            // All done.
            return Ok(());
        }

        // If we're still here, we need to truncate the directory item we were handed.
        
        
        // Go reset the first extent block
        let new_extent_start: FileExtentBlock = FileExtentBlock::new(first_extent);
        CachedBlockIO::update_block(&new_extent_start.to_block(), JustDiskType::Standard)?;
        
        // Update the inode
        // Set the file to a size of 0
        file.set_size(0);
        inode_with_file.file = Some(file);
        // Update the modification time
        inode_with_file.modified = InodeTimestamp::now();
        // Put the inode back in the block it came from, this will write for us.
        inode_block.update_inode(location.offset, inode_with_file)?;

        // All done!
        Ok(())
}

/// Just flushes the current FileExtentBlock to disk, nice helper function
fn flush_to_disk(block: &FileExtentBlock) -> Result<(), FloppyDriveError> {
    // Raw it
    let raw = block.to_block();
    // Write it.
    CachedBlockIO::update_block(&raw, JustDiskType::Standard)?;
    Ok(())
}

// local helper for new extents
impl FileExtent {
    fn new() -> Self {
        Self {
            flags: ExtentFlags::MarkerBit, // Marker bit must be set.
            disk_number: None,
            start_block: u16::MAX,
            length: 0,
        }
    }
}