// Writing files.

// We will take in InodeFile(s) instead of Extent related types, since we need info about how big files are so they are easier to extend.
// Creating files is handles on the directory side, since new files just have a name and location.

use std::{
    cmp::max,
    ops::{
        Div,
        Rem
    }
};

use log::{debug, warn};
use log::error;

use crate::{error_types::drive::DriveError, pool::{
    disk::{
        generic::{
            block::{
                block_structs::RawBlock,
                crc::add_crc_to_block
            },
            generic_structs::pointer_struct::DiskPointer,
            io::cache::cache_io::CachedBlockIO
        },
        standard_disk::block::{
                directory::directory_struct::{
                    DirectoryBlock, DirectoryItem, DirectoryItemFlags
                },
                file_extents::{
                    file_extents_methods::DATA_BLOCK_OVERHEAD,
                    file_extents_struct::{
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
            }
    },
    pool_actions::pool_struct::Pool
}, tui::{notify::NotifyTui, tasks::TaskType}};

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
    fn write(&mut self, bytes: &[u8], seek_point: u64) -> Result<u32, DriveError> {
       go_write(self, bytes, seek_point)
    }
}

impl DirectoryBlock {
    /// Create a new empty file in this directory with a name.
    /// 
    /// Adds the file to the directory, flushes it to disk.
    /// 
    /// Requires a mutable borrow, since this may update the block.
    /// 
    /// Returns the created file's directory item. Will contain disk info.
    /// 
    /// Should include extension iirc?
    pub fn new_file(&mut self, name: String) -> Result<DirectoryItem, DriveError> {
        go_make_new_file(self, name)
    }

    /// Deletes an file by deallocating every block the file used to take up, and removing it
    /// from the directory.
    /// 
    /// If you are looking to truncate a file, you need to call truncate() on the actual directory item.
    /// 
    /// Returns `None` if the file did not exist.
    ///
    /// Panics if fed a directory. Use remove_directory() !
    pub fn delete_file(&mut self, file: NamedItem) -> Result<Option<()>, DriveError> {
        // We only handle files here
        if !file.is_file() {
            // Why
            panic!("Cannot delete_file a non-file!");
        }
        
        // Extract the item
        let extracted_item: DirectoryItem;
        if let Some(exists) = self.find_and_extract_item(&file)? {
            // Item was there
            extracted_item = exists
        } else {
            // Tried to delete a file that does not exist in this directory.
            return Ok(None)
        };

        // Delete it.
        truncate_or_delete_file(&extracted_item, true, None)?;

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
    pub fn write_file(&self, bytes: &[u8], seek_point: u64) -> Result<u32, DriveError> {
        // We only handle files here
        if self.flags.contains(DirectoryItemFlags::IsDirectory) {
            // Why
            panic!("Cannot delete_file a non-file!");
        }

        // Extract out the file
        let location = &self.location;

        // Get the inode block
        let the_pointer_in_question: DiskPointer = location.pointer;

        let read: RawBlock = CachedBlockIO::read_block(the_pointer_in_question)?;
        let mut inode_block: InodeBlock = InodeBlock::from_block(&read);

        // Get the actual file.
        // The inode MUST exist.
        // A smarter filesystem would return the fact that the inode is missing and either try to rebuild it or
        // have the caller discard the item. But er, I dont have time.
        
        let inode_with_file = if let Ok(inode) = inode_block.try_read_inode(location.offset) {
            inode
        } else {
            // No inode...
            // Maybe some day, propagate that...
            // If this fails in the video, I'll write it lmao.
            panic!("No inode exists for this DirectoryItem's file. We cannot read it.");
        };

        // We already checked that this is a file.
        let mut file = if let Some(the_file) = inode_with_file.extract_file() {
            the_file
        } else {
            // ???
            panic!("File is a file, but not a file. Nice.");
        };

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

    /// Truncates a file to a specified byte length.
    /// 
    /// No action needs to be taken after this method.
    /// 
    /// Panics if fed a directory.
    pub fn truncate(&self, new_size: u64) -> Result<(), DriveError> {
        truncate_or_delete_file(self, false, Some(new_size))
    }
}

fn go_write(inode_file: &mut InodeFile, bytes: &[u8], seek_point: u64) -> Result<u32, DriveError> {
    let handle = NotifyTui::start_task(TaskType::FileWriteBytes, bytes.len() as u64);
    // Decompose the file into its pointers
    // No return location, we don't care where this puts us.
    let mut blocks = inode_file.as_pointers()?;

    // get the seek point
    let (block_index, mut byte_index) = InodeFile::byte_finder( seek_point);

    // The byte_finder already skips the flag, so it ends up adding one, we need to subtract that.
    byte_index -= 1;

    // Make sure we actually have a block at that offset. We cannot start writing from unallocated space.
    if block_index > blocks.len() {
        // Being told to write to a point we do not have.
        panic!("Attempted to write to unallocated space!");
    }
    
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
    if final_block_index > blocks.len() || blocks.is_empty() {
        // We need to get more room.
        // Special case will always allocate at least one block.
        let needed_blocks = max(final_block_index - blocks.len(), 1);

        // This should always be <= the max write size set in 
        // filesystem_methods.rs. Which should ALWAYS be quite far away
        // from u16::MAX, but we check anyways.

        // We hard cap it to u16 block though, just in case.
        if needed_blocks > u16::MAX.into() {
            // Crazy.
            panic!("Tried to write 2^16 blocks worth of data in one go! Not allowed!");
        }

        // Add that many more blocks to this file.
        // Since we know its already less than u16::MAX this cast is fine.
        let new_pointers = expand_file(*inode_file, needed_blocks as u16)?;

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
        NotifyTui::complete_multiple_task_steps(&handle, written as u64);
        // Keep going!
        continue;
    }

    // Done writing bytes!
    // Update the file size, only if we wrote past the end.
    let write_end = seek_point + bytes_written as u64;
    if write_end > inode_file.get_size() {
        inode_file.set_size(write_end);
    }

    NotifyTui::finish_task(handle);

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
fn update_block(block: DiskPointer, bytes: &[u8], offset: u16) -> Result<usize, DriveError> {

    // How much data a block can hold
    let data_capacity = 512 - DATA_BLOCK_OVERHEAD as usize;
    let offset = offset as usize;

    // Check for impossible offsets
    if offset >= data_capacity {
        panic!("Tried to write outside of the capacity of a block.");
    }
    
    // Calculate bytes to write based on REMAINING space.
    let remaining_space = data_capacity - offset;
    let bytes_to_write = std::cmp::min(bytes.len(), remaining_space);
    
    // We also don't support 0 byte writes.
    // Since that would be a failure mode of the caller, in theory could be
    // stuck in an infinite loop type shi.
    // Why exit? It won't if you fix the caller! :D
    if bytes_to_write == 0 {
        panic!("Tried to write 0 bytes to a block!");
    }

    // Now, if we are about to completely fill a block (ie every byte in the block will change)
    // we dont need to actually "update" the block, we can just replace it entirely.

    let mut block_copy = if bytes_to_write == data_capacity && offset == 0 {
        // Full block replacement, make fake block.

        // The flag byte never ended up being used, so we dont have to worry about it.
        RawBlock {
            block_origin: block,
            data: [0u8; 512], // Start with a blank slate
        }
    } else {
        // Partial update, still need to read in the old block.
        CachedBlockIO::read_block(block)?
    };
    
    // Modify that sucker
    // Skip the first byte with the flag
    let start = offset + 1;
    let end = start + bytes_to_write;

    block_copy.data[start..end].copy_from_slice(&bytes[..bytes_to_write]);

    // Update the crc
    add_crc_to_block(&mut block_copy.data);

    // Write that sucker
    CachedBlockIO::update_block(&block_copy)?;

    // Return the number of bytes we wrote.
    Ok(bytes_to_write)
}


/// Expands a file by adding `x` new blocks to the extents. Returns disk pointers for the new extents.
/// Updates underlying ExtentBlock(s) for this file.
/// 
/// May swap disks, does not return to any start disk.
fn expand_file(inode_file: InodeFile, blocks: u16) -> Result<Vec<DiskPointer>, DriveError> {
    debug!("Expanding a file by {blocks} blocks...");
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
fn expand_extent_block(block: &mut FileExtentBlock) -> Result<(), DriveError> {
    // Get a new block from the pool.
    // No need for crc, we will immediately write over it.
    let new_block_location = Pool::find_and_allocate_pool_blocks(1, false)?[0];

    // Put the a block there
    let new_block: RawBlock = FileExtentBlock::new(new_block_location).to_block();

    // Write, since we looked for a free block, didn't reserve it yet.
    CachedBlockIO::update_block(&new_block)?;

    // Now update the block we came in here with
    block.next_block = new_block_location;

    // Updated! All done.
    Ok(())
}

/// Will automatically deduce runs of blocks from a slice of disk pointers, then add those extents to the block,
/// expanding the block if needed.
/// 
/// We assume the incoming pointers are already sorted by disk, block. (1, 1), (1, 2), (2, 1) etc.
/// 
/// Does not check if blocks are already allocated, caller _MUST_ provide marked blocks.
fn expanding_add_extents(file: InodeFile, extents: &[FileExtent]) -> Result<(), DriveError> {
    // We will reverse the extents vec so we can pop them off the back
    // for easier adding, avoiding an index.
    let mut new_extents: Vec<FileExtent> = extents.to_vec();
    new_extents.reverse();

    // Go get the extent block to add to.
    // We need the final one in the chain.
    let mut current_extent_block: FileExtentBlock;
    
    // Read in the initial block
    let raw_read: RawBlock = CachedBlockIO::read_block(file.pointer)?;
    current_extent_block = FileExtentBlock::from_block(&raw_read);

    loop {
        // Is this the final block?
        if !current_extent_block.next_block.no_destination() {
            // No it isn't. We need to load the next block.
            // Get the block.
            let reader_mc_deeder: RawBlock = CachedBlockIO::read_block(current_extent_block.next_block)?;
            current_extent_block = FileExtentBlock::from_block(&reader_mc_deeder);
            // Try again.
            continue;
        }

        // This is the final block.
        // Try adding extents
        while let Some(last) = new_extents.last() {
            // Try adding a new extent.
            let added_result = current_extent_block.add_extent(*last);

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

    // Cant pre-allocate room in the vec, since we dont know how many extents we'll be creating,
    // and estimating it is hard.
    let mut new_extents: Vec<FileExtent> = Vec::new();
    
    // Loop over the pointers and create extents.
    for pointer in pointers {
        // Check if we need to make a new extent.
        // We start at 0 since we increment at the end of the loop.
        let new: FileExtent = FileExtent::new(*pointer, 0);
        // We need a new one if:
        // - There are no extents
        // - The disk number is different
        // - The length is maxed out
        // - The next block is not contiguous. (ie last block was 1, new block != 2)

        // yes this is ugly, at least it doesnt have to check for local disks anymore
        if let Some(extent) = new_extents.last() {
            if extent.start_block.disk != pointer.disk || // Is the disk number different?
            extent.length == u8::MAX || // Is this extent out of room?
            extent.start_block.block + extent.length as u16 != pointer.block // Non contiguous?
            {
                // Need a new one.
                new_extents.push(new);
            }
            // All checks pass, fall out.
        } else {
            // There isn't any extents yet, make the first one
            new_extents.push(new);
        }
        
        // This pointer extends the previous (or new) extent block, add one to the length.
        match new_extents.last_mut() {
            Some(last) => {
                last.length += 1;
            },
            None => {
                // I guess we never got any pointers.
                // Do nothing.
            },
        }
    }

    // All done!
    new_extents
}

/// Create a new file.
fn go_make_new_file(directory_block: &mut DirectoryBlock, name: String) -> Result<DirectoryItem, DriveError> {
    // Directory blocks already have a method to add a new item to them, so we just need
    // to create that item to add.

    // New files must have a filename that is <= u8::MAX.
    // Caller is in charge of checking this before giving it to us.
    if name.len() > u8::MAX.into() {
        panic!("File name was too long!");
    }

    // Timestamp for file creation
    let right_now: InodeTimestamp = InodeTimestamp::now();
    
    // No need for CRC, we will be writing over it.
    let in_progress = Pool::find_and_allocate_pool_blocks(1, false)?;
    let reserved_block: DiskPointer = in_progress[0];
    
    // Now that we have the new block we need a FileExtentBlock to write into it.
    let new_block: FileExtentBlock = FileExtentBlock::new(reserved_block);

    // No need to set the marker bit since this is a file ofc.

    // Now let's write that new block
    let raw: RawBlock = new_block.to_block();
    // Block is not marked as reserved, so this is a write.
    CachedBlockIO::update_block(&raw)?;

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

    let new_inode_location = Pool::fast_add_inode(new_inode)?;

    // Wrap it all up in a little bow to put into the directory

    // Now we need to set up the flags for the new directory item
    // Flag
    let flags: DirectoryItemFlags = DirectoryItemFlags::MarkerBit;
    // Thats it. Lol.

    // Construct the new file
    let new_file: DirectoryItem = DirectoryItem {
        flags,
        name_length: name.len() as u8,
        name,
        location: new_inode_location,
    };

    directory_block.add_item(&new_file)?;
    // If we're here, that worked. We are all done adding the item to the directory.

    // All done.
    // Dont need to swap disks, already did that on the item add.
    Ok(new_file)
}

// One hell of a function.
/// Will only truncate if delete is false.
fn truncate_or_delete_file(item: &DirectoryItem, delete: bool, new_size: Option<u64>) -> Result<(), DriveError> {
    // Is this a file?
    if item.flags.contains(DirectoryItemFlags::IsDirectory) {
        // Uh, no it isn't why did you give me a dir?
        panic!("Tried to truncate or delete a directory as if it was a file!");
    }

    // Load the size of the directory item
    let file_size: u64 = item.get_size()?;

    // If we aren't deleting, and the size is the same as the current size, we can skip truncation.
    if let Some(the_new_size) = new_size {
        // Make sure delete flag is not set.
        // In theory, None means delete is set.
        if !delete && the_new_size == file_size {
            // Skip
            return Ok(());
        }
    }

    // Extract out the file
    let file_inode_location = &item.location;

    // Get the inode block
    let the_pointer_in_question: DiskPointer = file_inode_location.pointer;

    let read: RawBlock = CachedBlockIO::read_block(the_pointer_in_question)?;
    let mut inode_block: InodeBlock = InodeBlock::from_block(&read);

    // Get the actual file
    let mut inode_with_file: Inode = if let Ok(inode) = inode_block.try_read_inode(file_inode_location.offset) {
        inode
    } else {
        // The inode for this file does not exist.
        panic!("Cannot truncate or delete files that do not have an inode!");
    };
    // We already checked that this is a file.
    let mut file: InodeFile = if let Some(the_file) = inode_with_file.extract_file() {
        the_file
    } else {
        // ?
        panic!("Flag for file set, but no file.");
    };

    // If the truncation is just growing the file, we can do this directly on the file itself without messing with the extents.

    // Truncation can also grow files, check if the truncation is larger than the current size.
    // Growing cannot happen at the same time as deletion.
    if let Some(extracted_new_size) = new_size {
        if file_size < extracted_new_size && !delete {
            // We are just growing.
            // Growing is easy, we just write zeros to make it the new size.

            // The difference
            let grow_size: usize = (extracted_new_size - file_size) as usize;
            
            // We need to do this in a loop, since would be consuming as much ram as the write is big, which isn't great.
            // So we will do it in 1MB chunks, but this may change in the future if its too slow.
            const CHUNK_SIZE: usize = 1024*1024;
            let zero_chunk: Vec<u8> = vec![0; CHUNK_SIZE];

            // Write to the end of the file with the zeros.
            let mut seek_point = file_size;
            for _ in 0..grow_size.div(CHUNK_SIZE) {
                // Yeah... Keep eating...
                let _ = item.write_file(&zero_chunk, seek_point)?;
                seek_point += CHUNK_SIZE as u64;
            }

            // Final write if there are any remaining bytes.
            let remainder = grow_size.rem(CHUNK_SIZE);
            if remainder != 0 {
                let final_zeros: Vec<u8> = vec![0; remainder];
                let _ = item.write_file(&final_zeros, seek_point)?;
            }
            
            // Make sure the new size is correct
            let gotten_size = item.get_size()?;
            if extracted_new_size != gotten_size {
                // Truncation did not work properly.
                error!("Truncated to the wrong size! Expected {extracted_new_size} got {gotten_size} !")
            };
            
            // All done.
            return Ok(());
        }
    }

    // If we are here, we must be shrinking or deleting.

    // If we are deleting, we can skip the more complicated extent logic.
    if delete {
        // We are deleting all of the blocks, so just get all of them.
        let mut used_blocks = file.as_pointers()?;

        // We also need to free the extent blocks themselves, not just where they point.
        let mut extent_block_pointer = file.pointer;

        while !extent_block_pointer.no_destination() {
            used_blocks.push(extent_block_pointer);
            // This work has already been done on `to_pointers`, maybe there should be another
            // method that returns the pointers, and the pointers to the extent blocks at the same time.
            // Not gonna write that tho, this is fine.
            let read = CachedBlockIO::read_block(extent_block_pointer)?;
            let extent_block: FileExtentBlock = FileExtentBlock::from_block(&read);
            extent_block_pointer = extent_block.next_block;
        }

        // We dont have to worry about updating the underlying block, since the deletion call
        // will discard the item automagically.

        // Sort the blocks to reduce swap
        used_blocks.sort_unstable_by_key(|block| (block.disk, block.block));

        // Chunk by disk.
        let chunked = used_blocks.chunk_by(|a, b| a.disk == b.disk);

        // Delete all the blocks by freeing all of them.
        for chunk in chunked {
            let _ = Pool::free_pool_block_from_disk(chunk)?;
        }

        // All done!
        return Ok(());
    }

    // Since we didn't delete, we must be shrinking, since we already checked for growing
    // This should be guarded.
    let new_size = new_size.expect("Cannot truncate a file without a size to truncate to.");


    // To truncate, several things need to happen:
    // - We need to update the data that is contained within the final data block to write in zeros
    // -  past the new ending size.
    // - We need to remove all of the data blocks that are no longer used.
    // - We need to remove all of the file extent blocks that are no longer used.
    // - We need to update the pointer on the new final extent block (if needed)
    // - - This will never point at a new block, since its the new final, and we cannot
    // - -  remove extents from the middle of a file.
    // - We need to remove all extents past this one in the new final file extent block.
    // - Update the file size.

    // I considered extracting out all of the extents to consolidate them again, but there would be
    // no space savings from doing that, since extents are always added at the end of the chain, and must be in order.
    // Thus, if an extent could have fit in the previous block, it would have already been there. You cannot truncate in
    // the middle of the file.
    // Thus, removing extents does not create gaps, and is already as efficient as possible (in Fluster's implementation at least, lol)

    // Steps:
    // Deduce what data block is the new final block.
    // - The FileExtentBlock that contained that block is the new final FileExtentBlock in the chain.
    // - The Extent that the data block was contained within is now the final Extent within the FileExtentBlock
    // Update the new final data block.
    // - Write zeros to the end of the block after the new file end point.
    // - Do not write it yet, it must be written after the ExtentBlock has been updated, otherwise we
    // -  would be changing file data without changing the file ending, corrupting the underlying file.
    // Remove all extents past this one in the containing extent block.
    // - Just pop off all ones after this one, and hold onto the pointers for them
    // -  so we can free those blocks later.
    // Update the pointer on the new final ExtentBlock
    // - Hold onto where it used to go, we need it for cleanup later
    // - Point to nowhere.
    // All at once:
    // - Flush the updated final extent to disk, then the updated data block.
    // - - Make sure to set the extents to be local if needed, since the disk gets set during the read process.
    // - - After this point, the file has been properly truncated from the filesystem perspective.
    // - - Even if the rest of the deletion fails after this point, we at least wont be pointing at the data blocks
    // - -  that were supposed to be freed. We'll have leaked them though, which stinks, but whatever.
    // - Update the file size
    // - Update the file modify time
    // Collect all of the DiskPointers to the remaining ExtentBlocks.
    // - Cant immediately remove them, we still need the DiskPointers to the data blocks
    // Collect all of the DiskPointers to the data blocks within the extents in the remaining extent blocks.
    // - Loop over the collected disk pointers in the previous step, open the FileExtentBlock, extract the
    // - DiskPointers from all of the contained Extents
    // Free all of the blocks we've collected
    

    // Is this a lot of documentation? yes.
    // Is this the third time i've rewritten this function from near-scratch today? Also yes.



    // == Deduce what data block is the new final block. ==

    // We need to find the block offset.
    let (new_final_block_index, new_final_block_byte_index) = InodeFile::byte_finder(new_size);

    // Now we can loop through the FileExtentBlocks, tracking how many data blocks we've seen so far
    // Get the first extent block
    let read: RawBlock = CachedBlockIO::read_block(file.pointer)?;
    let first_extent_block = FileExtentBlock::from_block(&read);

    // Go find the block, also keep track of what extent caused us to be full, since
    // that'll be our new final extent.
    let mut new_final_extent_block: FileExtentBlock = first_extent_block;
    // Dummy values that will be overwritten.
    let mut pointer_to_new_final_data_block: DiskPointer = DiskPointer::new_final_pointer();
    let mut offset_in_final_extent: usize = 0;
    let mut final_extent_index: usize = 0;

    let mut blocks_seen: usize = 0;
    let mut done: bool = false;

    // At this point, we know the truncation HAS to be less than the current size of the file, thus
    // we do not need to check if we run out of blocks, since they must be there.
    // If they aren't there, its a more fundamental issue, not our problem.
    loop {
        // Get the extents from the ExtentBlock
        let extents = new_final_extent_block.get_extents();

        // Now search through those extents, incrementing how many
        // blocks we've seen and keeping track of what extent this is
        for (index, extent) in extents.iter().enumerate() {
            // How many blocks are in here?
            let pointers: Vec<DiskPointer> = extent.get_pointers();
            // Add all of those pointers to the count
            blocks_seen += pointers.len();

            // Have we seen enough blocks?
            if blocks_seen >= new_final_block_index {
                // The last extent we opened is the final one!

                // We need to deduce which block it was, since we want to know the index of it.
                // Sure, we could have looped over the pointers instead of adding the length and just captured the disk pointer
                // by itself, but we still need to know the index as well, so we can remove the items after it later.
                // "erm what about iter().enumerate()" sybau ts pmo...

                // How many blocks we found - the number of blocks we wanted = how many extra blocks we read.
                // therefore, pointers.len() - extra = index into pointers where the block is

                // The number of blocks a extent can hold is at most 256, which fits into an i16 when negative.
                let offset = pointers.len() - (blocks_seen - new_final_block_index);
                offset_in_final_extent = offset;

                // Now we can get the pointer to the final data block
                pointer_to_new_final_data_block = pointers[offset];

                final_extent_index = index;
                done = true;
                break
            }
        }

        // Done?
        if done {
            break
        }
        
        // Need to keep going, get the next block.
        // Dont need to check if this is a final pointer, since we would crash if it was, and it shouldn't be.
        let read: RawBlock = CachedBlockIO::read_block(new_final_extent_block.next_block)?;
        let next = FileExtentBlock::from_block(&read);
        new_final_extent_block = next;
    }

    // == Update the new final data block. ==
    // == - Write zeros to the end of the block after the new file end point. ==
    // == - Do not write it yet, it must be written after the ExtentBlock has been updated, otherwise we ==
    // == -  would be changing file data without changing the file ending, corrupting the underlying file. ==

    // easy
    
    // Find the index into the block where everything past it will be blanked out...
    // jk, we already know that hehe, its in new_final_block_byte_index

    // Now load in the old block so we can update it
    let mut updated_final_data_block: RawBlock = CachedBlockIO::read_block(pointer_to_new_final_data_block)?;

    // Now blank it out.
    // Currently, the last 4 bytes of the block are the checksum. but since we're going to be updating the block anyways, we can write
    // over it, since we'll have to re-checksum it anyways.
    updated_final_data_block.data[new_final_block_byte_index as usize..].fill(0_u8);

    // Put the checksum back on
    add_crc_to_block(&mut updated_final_data_block.data);

    // Dont write the block yet, we'll hold onto it until _after_ we do the FileExtentBlock update.



    // == Remove all extents past this one in the containing extent block. ==
    // == - Just pop off all ones after this one, and hold onto the pointers for them ==
    // == -  so we can free those blocks later. ==

    // Start blocks that we need to free.
    // Cannot pre-allocate, since there's no way to know how many blocks we'll be freeing
    // at this point. Guesstimations could be made, but oh well.
    let mut blocks_to_free: Vec<DiskPointer> = Vec::new();

    // We also need to grab the extra disk pointers from the extent that holds the new
    // final data block, if there are any.

    // Update the extents for the final block.
    let mut updated_extents = new_final_extent_block.get_extents();

    for (index, extent) in updated_extents.iter_mut().enumerate() {
        // skip if this is before the final extent
        if index < final_extent_index {
            // skip
            continue;
        }
        // is this the final extent
        if index == final_extent_index {
            // Now we need to remove the extra disk pointers if there are any.
            let pointers = extent.get_pointers();
            // Split the vec to remove anything after the final data block pointer.
            // Splitting keeps `start..split`, but we need `start..=split` so we will need to
            // increment the index.
            let split_point: usize = offset_in_final_extent + 1;

            // Splitting will panic if this is past the end of the array. Which would be the case
            // if we found exactly as many blocks as we needed in the final extent.
            if split_point > pointers.len() {
                // We dont need to update this extent.
                continue;
            }

            // Do the splits, keeping the second section so we can get the pointers from it
            let to_free = pointers.split_at(split_point).1;

            // Now to update the extent, we just need to remove all items past the split point.
            // since extents are encoded as a start + length, we can just subtract the number of extra items.
            extent.length -= to_free.len() as u8;

            // Now add the extra pointers to the trash pile
            blocks_to_free.extend(to_free);
            
            // All done
            continue;
        }
        // This is after the last extent we care about, trash everything.
        let pointers = extent.get_pointers();
        blocks_to_free.extend(pointers);
    }

    // Now that we've collected all the extents we care about, and trashed everything else, we can drop any extra
    // extents from the block if they exist.
    // truncate is `..end` not `..=end` so we add one.
    let truncate_point: usize = final_extent_index + 1;
    updated_extents.truncate(truncate_point);

    // Now we only have the extents we care about in `updated_extents`.

    // == Update the pointer on the new final ExtentBlock ==
    // == - Hold onto where it used to go, we need it for cleanup later ==
    // == - Point to nowhere. ==
    
    // We have to point to nowhere before we can put in the new extents.
    // We will also hold onto it
    let unreferenced_extent_block_chain_start: DiskPointer = new_final_extent_block.next_block;
    new_final_extent_block.next_block = DiskPointer::new_final_pointer();

    // == All at once: ==
    // == - Flush the updated final extent to disk, then the updated data block. ==
    // == - - Make sure to set the extents to be local if needed, since the disk gets set during the read process. ==
    // == - - After this point, the file has been properly truncated from the filesystem perspective. ==
    // == - - Even if the rest of the deletion fails after this point, we at least wont be pointing at the data blocks ==
    // == - -  that were supposed to be freed. We'll have leaked them though, which stinks, but whatever. ==
    // == - Update the file size ==
    // == - Update the file modify time ==

    // Add the extents
    // This does not flush to disk.
    new_final_extent_block.force_replace_all_extents(updated_extents);

    // Now for the scary part.
    // This write must complete, followed by the update to the data block, otherwise data
    // will corrupt.

    let finished_extent_block: RawBlock = new_final_extent_block.to_block();

    debug!("Writing updated file extent block after size decrease...");
    debug!("If this fails, data corruption WILL occur..");
    
    // Oh boy.
    // We will do all 3 steps at once, even if the two writes fail, as long as the file size change works, the file
    // will _possibly_ be in a usable state, since bounds checks are done to make sure we dont read past the end.
    // ...Until something tries to extend the file, new blocks will pointlessly be added, and writing may skip over blocks,
    // resulting in the next read containing old data from pre-truncation.

    let extent_block_result = CachedBlockIO::update_block(&finished_extent_block);
    let data_block_result = CachedBlockIO::update_block(&updated_final_data_block);
    
    // Update the file
    file.set_size(new_size);
    inode_with_file.file = Some(file);
    // might as well set the time here too
    inode_with_file.modified = InodeTimestamp::now();
    // Write it back to the block it came from
    let inode_update_result = inode_block.update_inode(file_inode_location.offset, inode_with_file);
    

    // Now, did that all work?
    let all_worked: bool = extent_block_result.is_ok() && data_block_result.is_ok() && inode_update_result.is_ok();
    let all_failed: bool = extent_block_result.is_err() && data_block_result.is_err() && inode_update_result.is_err();
    if !all_worked && !all_failed { // at least one fail, but not all of them.
        // DAMNIT!

        // "why are you printing so much out here"
        // Well if the filesystem fails during my factorio run, this would
        // add extra drama hahaha... god i hope it doesn't fail...

        error!("TRUNCATION FAILURE!");
        error!("Listing what failed:");
        error!("==-==-==-==-==-==-==-==");
        if extent_block_result.is_err() {
            error!("- Extent block write.");
        }
        if data_block_result.is_err() {
            error!("- Data block write.");
        }
        if inode_update_result.is_err() {
            error!("- Inode update.");
        }
        error!("==-==-==-==-==-==-==-==");
        error!("We have to keep going. But this file may now be VERY unstable.");
        error!("Any further calls on this file will almost certainly do unexpected things.");

        // We have now leaked all of the blocks that we were intending to free.
        error!("At least `{}` blocks have now been leaked.", blocks_to_free.len());
        error!("That does not include:");
        error!("FileExtent blocks past the new final extent block in the extent chain.");
        error!("Data storing blocks that those FileExtent blocks pointed to.");
        // We can at least estimate it.
        // yes i know that %512 does not account for the flags and such in the data blocks,
        // but we arent counting the extent block overhead either so
        let leak_estimate: usize = (file_size - new_size).div_ceil(512) as usize;
        error!("Rough estimate: `{leak_estimate}` additional blocks leaked.");
        error!("Godspeed.");

        // If we are testing, this should panic as well.
        if cfg!(test) {
            panic!("Truncation fail.");
        }

        // No error, since we need to just ignore the error if we dont wanna completely give up
        return Ok(());
        
        // Too late to turn back, unless all 3 failed? but what are the odds?
    } else if all_failed {
        // Woah.
        // All three operations failed, which, unusually, is a good thing in this case.
        warn!("TRUNCATION FAILURE!");
        warn!("All of the operations failed, which means nothing was changed.");
        warn!("Scary, but we are actually fine in this case.");
        warn!("We can continue like nothing happened, because nothing did.");

        // This should also fail tests.
        if cfg!(test) {
            panic!("Truncation fail: No update.");
        }    

        // If the disk is busy, we retry, so...
        return Err(DriveError::Retry);
    } else {
        // Everything worked!
        debug!("Content update finished successfully. Phew.")
    }

    // Cool! Now we can go free blocks we don't need anymore.

    // Failure after this point will leak blocks. so we keep it locked up to report a leak.

    debug!("Running truncate cleanup...");
    let cleanup_result = truncate_cleanup(blocks_to_free, unreferenced_extent_block_chain_start);

    // Did that all sail smoothly
    match cleanup_result {
        Ok(free_count) => {
            // All good! All garbage has been freed!
            debug!("Truncate cleanup finished successfully.");
            debug!("Truncation freed {free_count} blocks.");
            Ok(())
        },
        Err(err) => {
            // Oh well.
            error!("Truncate cleanup did not finish!");
            error!("We have leaked blocks!");
            error!("Unknown how many leaked. But we can still safely continue.");
            error!("Failue: {err:#?}");
            // panic in tests.
            if cfg!(test) {
                panic!("Truncation cleanup fail.");
            }
            // We can keep going without caring, too bad about them wasted blocks tho, eh?
            Ok(())
        }
    }
}

/// Returns how many blocks were freed.
fn truncate_cleanup(pre_collected: Vec<DiskPointer>, next_extent_block: DiskPointer) -> Result<usize, DriveError> {
    // The rest of the cleanup happens in this function so we can easily check if any of it fails.
    debug!("Starting truncation cleanup, started with {} pre-collected blocks...", pre_collected.len());
    
    // Dump those pointers into a new local pile
    let mut to_free: Vec<DiskPointer> = pre_collected;

    // == Collect all of the DiskPointers to the remaining ExtentBlocks. ==
    // == - Cant immediately remove them, we still need the DiskPointers to the data blocks ==

    // Make sure the pointer goes somewhere, if it doesnt, we can skip this step.
    // Shadow it so we can update it for the loop
    let mut next_extent_block = next_extent_block;

    // Also collect the extents, gotta remember where the're from too
    // Cant pre-allocate this, no idea how many extent blocks there will be.
    let mut extents: Vec<(u16, FileExtent)> = Vec::new();

    debug!("Collecting blocks referred to by extents...");
    while !next_extent_block.no_destination() {
        // Open the extent
        let raw: RawBlock = CachedBlockIO::read_block(next_extent_block)?;
        let read: FileExtentBlock = FileExtentBlock::from_block(&raw);

        // get the extents
        let tents: Vec<(u16, FileExtent)> = read.get_extents().into_iter().map(|i|{
            (next_extent_block.disk, i)
        }).collect();

        extents.extend(tents);

        // Add this extent block to the pile
        to_free.push(next_extent_block);

        // set the next block
        next_extent_block = read.next_block;
    }

    debug!("Done.");
    
    // == Collect all of the DiskPointers to the data blocks within the extents in the remaining extent blocks. ==
    // == - Loop over the collected disk pointers in the previous step, open the FileExtentBlock, extract the ==
    // == - DiskPointers from all of the contained Extents ==
    
    // To save reads i grabbed the extents as i read the blocks, we just need the pointers
    debug!("Extracting pointers...");
    for (_, tent) in extents { // I used to be a tent, but I got too old and cant pitch them anymore.
        let pointers = tent.get_pointers();
        to_free.extend(pointers);
    }
    debug!("Done.");
    
    // == Free all of the blocks we've collected ==


    // Sort the blocks to reduce the amount of head seeking. This also groups together the disks.
    to_free.sort_unstable_by_key(|block| (block.disk, block.block));

    // Make sure there are no duplicates.
    // Yes we shouldn't be getting them in the first place, but if we somehow do, this will
    // crash due to double free.
    let pre_dedup = to_free.len();
    to_free.dedup();
    let post_dedup = to_free.len();
    if pre_dedup != post_dedup {
        // Sizes are different, cooked.
        panic!("There was duplicate blocks during truncation cleanup. Cannot continue.");
    }

    // Hold onto how many blocks we're freeing for returning.
    let amount_freed = to_free.len();

    // Split into sections based on when the disk changes.
    let chunked = to_free.chunk_by(|a, b| a.disk == b.disk);

    // Now go free all of those blocks.
    // This will zero out the blocks, and remove them from the cache for us.
    debug!("Freeing blocks...");
    for chunk in chunked {
        let _ = Pool::free_pool_block_from_disk(chunk)?;
    }
    debug!("Done.");
    debug!("All done cleaning up truncation.");

    // All done! Return how many blocks we freed.
    Ok(amount_freed)
}

/// Just flushes the current FileExtentBlock to disk, nice helper function
fn flush_to_disk(block: &FileExtentBlock) -> Result<(), DriveError> {
    // Raw it
    let raw = block.to_block();
    // Write it.
    CachedBlockIO::update_block(&raw)?;
    Ok(())
}