// We need to go to seek points and such.

use log::debug;

use crate::{error_types::drive::DriveError, pool::disk::{
    generic::{
        block::block_structs::RawBlock,
        generic_structs::pointer_struct::DiskPointer,
        io::cache::cache_io::CachedBlockIO
    },
    standard_disk::block::{
            directory::directory_struct::DirectoryItem,
            file_extents::file_extents_methods::DATA_BLOCK_OVERHEAD,
            inode::inode_struct::{
                Inode,
                InodeBlock,
                InodeFile
            }
        }
}};

impl InodeFile {
    /// Find where a seek lands.
    /// Returns (index, offset), index is the index into the input blocks array,
    /// offset is the offset within that block, skipping the flag byte already.
    #[inline]
    pub(super) fn byte_finder(byte_offset: u64) -> (usize, u16) {
        // Assumptions:
        // We aren't attempting to find a byte offset that is outside of the file.

        let block_capacity = 512 - DATA_BLOCK_OVERHEAD;

        // We can divide the incoming offset by the block capacity to figure out which block it's in.
        // This gives the index into the `blocks` slice directly.
        let block_index = (byte_offset / block_capacity) as usize;

        // Now within that block we can find which byte it is by taking the modulo.
        // But we do need to move forwards one byte into the block to skip the flag.
        let offset_in_block = (byte_offset % block_capacity) as u16 + 1;

        // All done!
        (block_index, offset_in_block)
    }
}

impl DirectoryItem {
    /// Retrieve the inode that refers to this block.
    pub fn get_inode(&self) -> Result<Inode, DriveError> {
        debug!("Extracting inode from DirectoryItem...");
        // read in that inode block
        let pointer: DiskPointer = self.location.pointer;
        
        debug!("Reading in InodeBlock at (disk {} block {})...", pointer.disk, pointer.block);
        let raw_block: RawBlock = CachedBlockIO::read_block(pointer)?;
        let block: InodeBlock = InodeBlock::from_block(&raw_block);
        
        // return the inode
        let inode_good = block.try_read_inode(self.location.offset).expect("Don't feed this invalid offsets! hehehe");
        debug!("Inode found.");
        Ok(inode_good)
    }
}