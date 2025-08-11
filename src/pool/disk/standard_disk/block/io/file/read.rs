// Reading a block is way easier than writing it.
// Must use cached IO, does not touch disk directly.

use log::{debug, trace};

use crate::pool::disk::{
    drive_struct::FloppyDriveError,
    generic::{
        block::block_structs::RawBlock,
        generic_structs::pointer_struct::DiskPointer,
        io::cache::cache_io::CachedBlockIO
    },
    standard_disk::{
        block::{
            directory::directory_struct::{
                DirectoryFlags,
                DirectoryItem
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
    }
};

impl InodeFile {
    // Local functions
    /// Extract all of the extents and spit out a list of all of the blocks.
    pub(super) fn to_pointers(&self) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        go_to_pointers(self)
    }
    /// Extract all of the extents.
    pub(super) fn to_extents(&self) -> Result<Vec<FileExtent>, FloppyDriveError> {
        let root = self.get_root_block()?;
        go_to_extents(&root)
    }
    /// Goes and gets the FileExtentBlock this refers to.
    fn get_root_block(&self) -> Result<FileExtentBlock, FloppyDriveError> {
        go_get_root_block(self)
    }
    /// Read a file
    fn read(&self, seek_point: u64, size: u32) -> Result<Vec<u8>, FloppyDriveError> {
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
    pub fn read_file(&self, seek_point: u64, size: u32) -> Result<Vec<u8>, FloppyDriveError> {
        // Is this a file?
        if self.flags.contains(DirectoryFlags::IsDirectory) {
            // Uh, no it isn't why did you give me a dir?
            panic!("Tried to read a directory as a file!")
        }

        // Extract out the file
        assert!(self.location.disk.is_some());
        let location = &self.location;

        // Get the inode block
        let pointer: DiskPointer = DiskPointer {
            disk: location.disk.expect("Assumption 2"),
            block: location.block,
        };

        let raw_block: RawBlock = CachedBlockIO::read_block(pointer)?;
        let inode_block: InodeBlock = InodeBlock::from_block(&raw_block);

        // Get the actual file
        let inode_file = inode_block.try_read_inode(location.offset).expect("Caller guarantee.");
        let file = inode_file.extract_file().expect("Caller guarantee.");

        // Now we can read in the file
        let read_bytes = file.read(seek_point, size,)?;

        // Now we have the bytes. If we were writing, we would have to flush info about the file to disk, but we don't
        // need to for a read. We are all done

        Ok(read_bytes)
    }
}



fn go_to_pointers(location: &InodeFile) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    // get extents
    let extents = location.to_extents()?;
    // Extract all the blocks
    let mut blocks: Vec<DiskPointer> = Vec::new();

    // For each extent
    for e in extents {
        // each block that the extent references
        for n in 0..e.length {
            blocks.push(DiskPointer {
                disk: e.disk_number.expect("Read extents should have disk"),
                block: e.start_block + n as u16
            });
        }
    }

    Ok(blocks)
}

// Functions

fn go_to_extents(
    block: &FileExtentBlock,
) -> Result<Vec<FileExtent>, FloppyDriveError> {
    // Totally didn't just lift the directory logic and tweak it, no sir.
    debug!("Extracting extents for a file...");
    // We need to iterate over the entire ExtentBlock chain and get every single item.
    // We assume we are handed the first ExtentBlock in the chain.
    let mut extents_found: Vec<FileExtent> = Vec::new();
    let mut current_dir_block: FileExtentBlock = block.clone();
    // To keep track of what disk an extent is from
    let mut current_disk: u16 = block.block_origin.disk;

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Add all of the contents of the current directory to the total
        // But we will add the disk location data to these structs, it is the responsibility of the caller
        // to remove these disk locations if they no longer need them.
        // Otherwise if we didn't add the disk location for every item, it would be impossible
        // to know where a local pointer goes.
        let mut new_items = current_dir_block.get_extents();
        for item in &mut new_items {
            // If the disk location is already there, we wont do anything.
            if item.disk_number.is_none() {
                // There was no disk information, it must be local.
                item.disk_number = Some(current_disk)
            }
            // Otherwise there was already a disk being pointed to.
            // Overwriting it here would corrupt it.
        }

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

        // Update what disk we're on
        current_disk = next_block.disk;
        let raw_block: RawBlock = CachedBlockIO::read_block(next_block)?;
        current_dir_block = FileExtentBlock::from_block(&raw_block);

        // Onwards!
        continue;
    }

    // We will not sort this vec, since the order matters. The blocks are added to extend the file always at the end.
    // TODO: Assert that this is true ^

    debug!("Extents retrieved.");
    Ok(extents_found)
}


fn go_get_root_block(file: &InodeFile) -> Result<FileExtentBlock, FloppyDriveError> {
    // Make sure this actually goes somewhere
    assert!(!file.pointer.no_destination());
    let raw_block: RawBlock = CachedBlockIO::read_block(file.pointer)?;
    let block = FileExtentBlock::from_block(&raw_block);
    Ok(block)
}



fn go_read_file(file: &InodeFile, seek_point: u64, size: u32) -> Result<Vec<u8>, FloppyDriveError> {
    // Make sure the file is big enough
    assert!(file.get_size()>= seek_point + size as u64);

    // Find the start point
    let (block_index, mut byte_index) = InodeFile::byte_finder( seek_point);

    // The byte_finder already skips the flag, so it ends up adding one, we need to subtract that.
    // TODO: This is a bandaid fix. this logic is ugly.
    byte_index -= 1;

    let blocks = file.to_pointers()?;
    let mut bytes_remaining: u32 = size;
    let mut current_block: usize = block_index;
    let mut collected_bytes: Vec<u8> = Vec::new();

    // We dont need to deal with the disk at all at this level, we will use
    // the cache for all IO

    loop {
        // Are we done reading?
        if bytes_remaining == 0 {
            // All done!
            break
        }
        let mut read_bytes = read_bytes_from_block(blocks[current_block], byte_index, bytes_remaining)?;
        // After the first read, we are now aligned to the start of blocks
        byte_index = 0;

        // Update how many bytes we've read
        bytes_remaining -= read_bytes.len() as u32;

        // add to the bucket
        collected_bytes.append(&mut read_bytes);
        // Keep going!
        current_block += 1;
        continue;
    }

    // All done!
    Ok(collected_bytes)
}





/// Read as many bytes as we can from this block.
/// 
/// Returns number of bytes read
fn read_bytes_from_block(block: DiskPointer, offset: u16, bytes_to_read: u32) -> Result<Vec<u8>, FloppyDriveError> {

    // How much data a block can hold
    let data_capacity = 512 - DATA_BLOCK_OVERHEAD as usize;
    let offset = offset as usize;

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

    let read_bytes: Vec<u8> = Vec::from(&block_copy.data[start..end]);

    // Return the bytes we read.
    Ok(read_bytes)
}