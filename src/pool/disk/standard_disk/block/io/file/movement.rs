// We need to go to seek points and such.

use crate::pool::disk::{generic::generic_structs::pointer_struct::DiskPointer, standard_disk::block::{file_extents::{file_extents_methods::DATA_BLOCK_OVERHEAD, file_extents_struct::{FileExtent, FileExtentBlock}}, inode::inode_struct::InodeFile, io::file::types::DataBytePointer}};

impl InodeFile {
    /// Find where a seek lands.
    /// Takes in a Vec<FileExtent> of every extent within the chain of blocks.
    pub(super) fn byte_finder(extents: &[FileExtent], byte_offset: u64) -> DataBytePointer {
        // There might be smarter ways to do this, but this should be fast enough.
        
        // Assumptions:
        // The list of FileExtents is ordered.
        // 
        // We aren't attempting to find a byte offset that is outside of the file.
        // 
        // The input list of file extents has all of the disks set. We dont want to manage
        // local pointers here.
        
        // We can divide the incoming offset by the block capacity to figure out which block its in.
        // This division rounds down.
        let containing_block_index = byte_offset / (512 - DATA_BLOCK_OVERHEAD);
        
        // Now within that block we can find which byte it is by taking the modulo.
        // But we do need to move forwards one byte into the block to skip the flag.
        let containing_block_offset: u16 = ((byte_offset % (512 - DATA_BLOCK_OVERHEAD)) + 1).try_into().expect("This shouldn't go above 512 - overhead.");

        // Now we need to find the actual block that ended up at, so we must deduce the locations
        // of extent blocks until we have have a block that matches our index.

        let mut blocks_seen: u64 = 0;

        for extent in extents {
            if containing_block_index <= blocks_seen + extent.length as u64 {
                // The block is within this extent.
                let block_offset =  containing_block_index - blocks_seen;
                return DataBytePointer {
                    disk: extent.disk_number.expect("Caller should set disk numbers."),
                    block: extent.start_block + block_offset as u16,
                    offset: containing_block_offset,
                }
            }
            // Wasn't in this extent. Increment and try again.
            blocks_seen += extent.length as u64;
            continue;
        }

        // If we made it past the loop, that means we ran out of extents to check, thus the requested
        // offset must have been outside of the file.
        panic!("Attempted to find a byte in a list of file extents shorter than the total offset!");
    }
}