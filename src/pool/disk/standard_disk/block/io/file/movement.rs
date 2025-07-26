// We need to go to seek points and such.

use crate::pool::disk::{generic::generic_structs::pointer_struct::DiskPointer, standard_disk::block::{file_extents::file_extents_methods::DATA_BLOCK_OVERHEAD, inode::inode_struct::InodeFile}};

impl InodeFile {
    /// Find where a seek lands.
    /// Returns (index, offset), index is the index into the input blocks array,
    /// offset is the offset within that block, skipping the flag byte already.
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