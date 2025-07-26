// Writing files.

// We will take in InodeFile(s) instead of Extent related types, since we need info about how big files are so they are easier to extend.
// Creating files is handles on the directory side, since new files just have a name and location.

use std::cmp::max;

use crate::pool::{disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{block::{block_structs::RawBlock, crc::add_crc_to_block}, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::{block::{directory::directory_struct::DirectoryBlock, file_extents::{file_extents_methods::DATA_BLOCK_OVERHEAD, file_extents_struct::{ExtentFlags, FileExtent, FileExtentBlock, FileExtentBlockFlags}}, inode::inode_struct::InodeFile}, standard_disk_struct::StandardDisk}}, pool_actions::pool_struct::Pool};

impl InodeFile {
    /// Update the contents of a file starting at the provided seek point.
    /// Will automatically grow file if needed.
    /// 
    /// Optionally returns to a provided disk when done.
    /// 
    /// Returns number of bytes written.
    fn write(self, bytes: &[u8], seek_point: u64, return_to: Option<u16>) -> Result<u64, FloppyDriveError> {
       go_write(self, bytes, seek_point, return_to)
    }
    /// Deletes a file by deallocating every block the file used to take up, including
    /// all of the FileExtent blocks that were used to construct the file.
    fn delete(self) -> Result<(), FloppyDriveError> {
        todo!();
    }
    /// Truncates a file. Deallocates every block that used to hold data for this file.
    /// Does not delete the origin FileExtent block.
    fn truncate(&mut self) -> Result<(), FloppyDriveError> {
        todo!();
    }
}

impl DirectoryBlock {
    /// Create a new empty file in this directory with a name.
    /// 
    /// Adds the file to the directory, flushes it to disk.
    /// 
    /// Returns the created file.
    /// 
    /// Should include extension iirc?
    fn new_file(self, name: String) -> InodeFile {
        todo!();
    }
}

fn go_write(inode: InodeFile, bytes: &[u8], seek_point: u64, return_to: Option<u16>) -> Result<u64, FloppyDriveError> {
    // Decompose the file into its pointers
    // No return location, we don't care where this puts us.
    let mut blocks = inode.to_pointers(None)?;

    // get the seek point
    let (block_index, mut byte_index) = InodeFile::byte_finder( seek_point);

    // Make sure we actually have a block at that offset. We cannot start writing from unallocated space.
    assert!(block_index <= blocks.len());
    
    // Now we can calculate where the final byte of this write will end up.
    // Minus 1, since we are writing to the byte we start the seek from
    // IE: if we write 1 byte from out offset, we don't actually move forwards into the next byte.
    let (final_block_index, _) = InodeFile::byte_finder(seek_point + bytes.len() as u64 - 1);

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
        let new_pointers = expand_file(inode, needed_blocks.try_into().expect("Guarded."))?;

        // The new pointers are already in order for us, and we will add them onto the end of the
        // pointers we grabbed earlier from the file.
        blocks.extend(new_pointers.iter());
    }

    // Now we know we have enough space for this write, let's get started.

    let mut bytes_written: usize = 0;
    let mut floppy_disk: StandardDisk;
    // Since the write can start un-aligned, we need to use an offset until its aligned again.
    let mut byte_write_index: u16 = byte_index;

    // Load in the first block
    floppy_disk = match FloppyDrive::open(blocks[block_index].disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("We should never be given a block on a non-standard disk."),
    };

    // Now we will loop through the blocks starting at the current index
    for block in &blocks[block_index..] {
        // are we out of bytes to write?
        if bytes_written == bytes.len() {
            // All done!
            break
        }
        // Do we need to switch disks?
        if block.disk != floppy_disk.number {
            // Need to swap.
            let new_disk = match FloppyDrive::open(block.disk)? {
                DiskType::Standard(standard_disk) => standard_disk,
                _ => unreachable!("Pool should never return a non-standard disk for new blocks."),
            };
            floppy_disk = new_disk;
        }
        // Update the block
        let written = update_block(*block, &bytes[bytes_written..], byte_index)?;
        // After the first write, the offset should be fixed now, since we've either written all of our bytes, in
        // which case we would be done, or we ran out of room in the block, thus the next block's offset would be 0.
        byte_index = 0;
        // Update how many bytes we've written
        bytes_written += written;
        // Keep going!
        continue;
    }

    // Done writing bytes!

    // Do we need to return to a disk?
    if let Some(returning) = return_to {
        let _ = FloppyDrive::open(returning)?;
    }

    // Return how many bytes we wrote!
    Ok(bytes_written as u64)
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
    let mut disk = match FloppyDrive::open(block.disk)? {
        crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("How are we reading a block from a non-standard disk?"),
    };
    let mut block_copy = disk.checked_read(block.block)?;
    
    // Modify that sucker
    // Skip the first byte with the flag
    let start = offset + 1;
    let end = start + bytes_to_write;

    block_copy.data[start..end].copy_from_slice(&bytes[..bytes_to_write]);

    // Update the crc
    add_crc_to_block(&mut block_copy.data);

    // Write that sucker
    disk.checked_update(&block_copy)?;

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
    let reserved_blocks = Pool::find_and_allocate_pool_blocks(blocks)?;

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
fn expand_extent_block(block: &mut FileExtentBlock, current_disk: &mut StandardDisk) -> Result<(), FloppyDriveError> {
    // Get a new block from the pool
    let the_finder = Pool::find_free_pool_blocks(1)?;
    let new_block_location = the_finder.last().expect("Asked for 1.");

    // Put the a block there
    let new_block: RawBlock = FileExtentBlock::new().to_block(new_block_location.block);

    // Are we already on the right disk?
    let caller_disk: u16 = current_disk.number;
    let new_location_disk: u16 = new_block_location.disk;
    if caller_disk == new_location_disk {
        // Need to move.
        let new_disk = match FloppyDrive::open(new_location_disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Pool should never return a non-standard disk for new blocks."),
        };
        *current_disk = new_disk;
    }

    // write the new block.
    // Write, since we looked for a free block, didn't reserve it yet.
    current_disk.checked_write(&new_block)?;

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
    let mut current_disk = file.pointer.disk;
    let mut current_block = file.pointer.block;
    let mut current_extent_block: FileExtentBlock;
    let mut floppy_disk: StandardDisk;

    // Load in the first block
    floppy_disk = match FloppyDrive::open(current_disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("We should never be given a block on a non-standard disk."),
    };

    current_extent_block = FileExtentBlock::from_block(&floppy_disk.checked_read(current_block)?);

    loop {
        // Is this the final block?
        if !current_extent_block.next_block.no_destination() {
            // No it isn't. We need to load the next block.

            // new disk?
            if current_disk != current_extent_block.next_block.disk {
                // Need to load in the new disk.
                floppy_disk = match FloppyDrive::open(current_disk)? {
                    DiskType::Standard(standard_disk) => standard_disk,
                    _ => unreachable!("We should never be given a block on a non-standard disk."),
                };
                current_disk = current_extent_block.next_block.disk;
            }
            // Get the block.
            current_block = current_extent_block.next_block.block;
            current_extent_block = FileExtentBlock::from_block(&floppy_disk.checked_read(current_block)?);
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
            flush_to_disk(&current_extent_block, &mut floppy_disk)?;
            // bye
            break
        }

        // We must've ran out of room.
        // Expand the block please.
        expand_extent_block(&mut current_extent_block, &mut floppy_disk)?;
        
        // Now we must write that extended block to disk.
        flush_to_disk(&current_extent_block, &mut floppy_disk)?;

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
            // Same disk, increment block length.
            current_extent.length += 1
        }
        // If the disk number doesn't match, or we cant add to the length anymore, time for a new extent.
        if current_extent.disk_number != Some(pointer.disk) || current_extent.length == u8::MAX {
            // push the current extent
            new_extents.push(current_extent);
            // clear the current extent so we can start over.
            current_extent = FileExtent::new()
        }
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

/// Just flushes the current FileExtentBlock to disk, nice helper function
fn flush_to_disk(block: &FileExtentBlock, current_disk: &mut StandardDisk) -> Result<(), FloppyDriveError> {
    // Raw it
    let raw = block.to_block(block.block_origin.block);
    // Write it.
    current_disk.checked_update(&raw)?;
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