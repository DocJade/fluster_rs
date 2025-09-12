// Reading a block is way easier than writing it.
// Must use cached IO, does not touch disk directly.


use log::{
    debug,
    trace
};

use crate::{error_types::drive::DriveError, pool::disk::{
    generic::{
        block::block_structs::RawBlock,
        generic_structs::pointer_struct::DiskPointer,
        io::cache::cache_io::CachedBlockIO
    },
    standard_disk::block::{
            directory::directory_struct::{
                DirectoryItem, DirectoryItemFlags
            },
            file_extents::{
                file_extents_methods::DATA_BLOCK_OVERHEAD,
                file_extents_struct::{
                    FileExtent,
                    FileExtentBlock
                }
            },
            inode::inode_struct::{
                InodeBlock,
                InodeFile
            }
        }
}, tui::{notify::NotifyTui, tasks::TaskType}};

impl InodeFile {
    // Local functions
    /// Extract all of the extents and spit out a list of all of the blocks.
    pub(super) fn as_pointers(&self) -> Result<Vec<DiskPointer>, DriveError> {
        go_to_pointers(self)
    }
    /// Extract all of the extents.
    pub(super) fn as_extents(&self) -> Result<Vec<FileExtent>, DriveError> {
        let root = self.get_root_block()?;
        go_to_extents(&root)
    }
    /// Goes and gets the FileExtentBlock this refers to.
    fn get_root_block(&self) -> Result<FileExtentBlock, DriveError> {
        go_get_root_block(self)
    }
    /// Read a file
    fn read(&self, seek_point: u64, size: u32) -> Result<Vec<u8>, DriveError> {
        go_read_file(self, seek_point, size)
    }
}

// We dont want to call read/write on the inodes, we should do it up here so we
// we can automatically update the information on the file, and the directory if needed.
impl DirectoryItem {
    /// Read a file.
    ///
    /// Assumptions:
    /// - This is an FILE, not a DIRECTORY.
    /// - The location of this directory item has it's disk set.
    /// - The inode that the item points at does exist, and is valid.
    /// 
    /// Reads in a file at a starting offset, and returns `x` bytes after that offset.
    /// 
    /// Optionally returns to a specified disk.
    pub fn read_file(&self, seek_point: u64, size: u32) -> Result<Vec<u8>, DriveError> {
        // Is this a file?
        if self.flags.contains(DirectoryItemFlags::IsDirectory) {
            // Uh, no it isn't why did you give me a dir?
            panic!("Tried to read a directory as a file!");
        }

        // Extract out the file
        let location = &self.location;

        // Get the inode block
        let pointer: DiskPointer = location.pointer;

        let raw_block: RawBlock = CachedBlockIO::read_block(pointer)?;
        let inode_block: InodeBlock = InodeBlock::from_block(&raw_block);

        // Get the actual file
        let inode_file = inode_block.try_read_inode(location.offset).expect("Already checked if it was a file.");
        let file = inode_file.extract_file().expect("File flag means a file inode should exist.");

        // Now we can read in the file
        let read_bytes = file.read(seek_point, size,)?;

        // Now we have the bytes. If we were writing, we would have to flush info about the file to disk, but we don't
        // need to for a read. We are all done

        Ok(read_bytes)
    }
}



fn go_to_pointers(location: &InodeFile) -> Result<Vec<DiskPointer>, DriveError> {
    // get extents
    let extents = location.as_extents()?;
    // Extract all the blocks.
    // Pre-allocating this vec isn't really possible, but we at least know that
    // every extent will contain at least one block.
    let mut blocks: Vec<DiskPointer> = Vec::with_capacity(extents.len());

    // For each extent
    for e in extents {
        // each block that the extent references
        for n in 0..e.length {
            blocks.push(DiskPointer {
                disk: e.start_block.disk,
                block: e.start_block.block + n as u16
            });
        }
    }

    Ok(blocks)
}

// Functions

fn go_to_extents(
    block: &FileExtentBlock,
) -> Result<Vec<FileExtent>, DriveError> {
    // Totally didn't just lift the directory logic and tweak it, no sir.
    debug!("Extracting extents for a file...");
    // We need to iterate over the entire ExtentBlock chain and get every single item.
    // We assume we are handed the first ExtentBlock in the chain.
    // Cannot pre-allocate here, since we have no idea how many extents there will be.
    let mut extents_found: Vec<FileExtent> = Vec::new();
    let mut current_dir_block: FileExtentBlock = block.clone();

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Add all of the contents of the current directory to the total.
        let new_items = current_dir_block.get_extents();
        extents_found.extend_from_slice(&new_items);

        // I want to get off Mr. Bone's wild ride
        if current_dir_block.next_block.no_destination() {
            // We're done!
            trace!("Done getting FileExtent(s).");
            break;
        }

        trace!("Need to continue on the next block.");
        // Time to load in the next block.
        let next_block = current_dir_block.next_block;
        let raw_block: RawBlock = CachedBlockIO::read_block(next_block)?;
        current_dir_block = FileExtentBlock::from_block(&raw_block);

        // Onwards!
        continue;
    }

    debug!("Extents retrieved.");
    Ok(extents_found)
}


fn go_get_root_block(file: &InodeFile) -> Result<FileExtentBlock, DriveError> {
    // Make sure this actually goes somewhere
    assert!(!file.pointer.no_destination(), "Pointer with no destination!");
    let raw_block: RawBlock = CachedBlockIO::read_block(file.pointer)?;
    let block = FileExtentBlock::from_block(&raw_block);
    Ok(block)
}



fn go_read_file(file: &InodeFile, seek_point: u64, size: u32) -> Result<Vec<u8>, DriveError> {
    let handle = NotifyTui::start_task(TaskType::FileReadBytes, size.into());
    // Make sure the file is big enough
    assert!(file.get_size()>= seek_point + size as u64, "Not enough bytes in this file to satisfy the read!");

    // Find the start point
    let (block_index, mut byte_index) = InodeFile::byte_finder( seek_point);

    // The byte_finder already skips the flag, so it ends up adding one, we need to subtract that.
    // This is a bandaid fix. this logic is ugly.
    // Not gonna refactor it tho, hehe.
    byte_index -= 1;

    let blocks = file.as_pointers()?;
    let mut bytes_remaining: u32 = size;
    let mut current_block: usize = block_index;

    // Since we will be writing into this vec, we need to pre-fill it with zeros to allow for indexing.
    // Doing it like this also avoids needing to grow the vec with additional data.
    let mut collected_bytes: Vec<u8> = vec![0_u8; size as usize];

    // We dont need to deal with the disk at all at this level, we will use
    // the cache for all IO

    loop {
        // Are we done reading?
        if bytes_remaining == 0 {
            // All done!
            break
        }

        // Get where the next bytes need to go
        let append_point = (size - bytes_remaining) as usize;

        // Read into the buffer
        let bytes_read = read_bytes_from_block(&mut collected_bytes, append_point, blocks[current_block], byte_index, bytes_remaining)?;
        
        // After the first read, we are now aligned to the start of blocks
        byte_index = 0;

        // Update how many bytes we've read
        bytes_remaining -= bytes_read as u32;
        NotifyTui::complete_multiple_task_steps(&handle, bytes_read.into());

        // Keep going!
        current_block += 1;
        continue;
    }

    NotifyTui::finish_task(handle);

    // All done!
    Ok(collected_bytes)
}





/// Read as many bytes as we can from this block.
/// 
/// Buffer must have enough room for our write. MUST pre-allocate it.
/// 
/// buffer_offset is how far into the provided buffer to append the newly read bytes.
/// 
/// Places read bytes into the provided buffer.
/// 
/// Returns number of bytes read.
fn read_bytes_from_block(buffer: &mut [u8], buffer_offset: usize, block: DiskPointer, internal_block_offset: u16, bytes_to_read: u32) -> Result<u16, DriveError> {

    // How much data a block can hold
    let data_capacity = 512 - DATA_BLOCK_OVERHEAD as usize;
    let offset = internal_block_offset as usize;

    // Check for impossible offsets
    assert!(offset < data_capacity, "Tried to read outside of the capacity of a block.");
    
    // Calculate bytes to write based on REMAINING space.
    let remaining_space = data_capacity - offset;
    let bytes_to_read = std::cmp::min(bytes_to_read, remaining_space as u32);
    
    // We also don't support 0 byte reads
    // Since that would be a failure mode of the caller, in theory could be
    // stuck in an infinite loop type shi.
    // Why panic? It won't if you fix the caller! :D
    assert_ne!(bytes_to_read, 0, "Tried to read 0 bytes from a block!");


    // load the block
    let block_copy: RawBlock = CachedBlockIO::read_block(block)?;
    
    // Read that sucker
    // Skip the first byte with the flag
    let start = offset + 1;
    let end = start + bytes_to_read as usize;
    let amount_read: usize = end - start;

    // Put the bytes into the buffer that was passed in.

    // Create slices for the buffer and the data, zero cost abstraction i think, just makes
    // code prettier.

    let destination_slice = &mut buffer[buffer_offset..buffer_offset + amount_read];
    let block_data = &block_copy.data[start..end];

    destination_slice.copy_from_slice(block_data);

    // Return the bytes we read.
    // No way to read more than 512 bytes, so u16 is fine
    Ok(amount_read as u16)
}