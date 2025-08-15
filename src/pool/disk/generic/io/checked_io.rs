// IO operations that ensure allocations are properly set.
// We panic in here if we try to read/write in an invalid way, since that indicates a logic error elsewhere.

use log::trace;

use crate::pool::{
    disk::{drive_struct::FloppyDriveError, generic::{
        block::{
            allocate::block_allocation::BlockAllocation,
            block_structs::{BlockError, RawBlock},
        },
        disk_trait::GenericDiskMethods, generic_structs::pointer_struct::DiskPointer,
    }},
    pool_actions::pool_struct::GLOBAL_POOL,
};

// A fancy new trait thats built out of other traits!
// Automatically add it to all types that implement the subtypes we need.
impl<T: BlockAllocation + GenericDiskMethods> CheckedIO for T {}
pub(super) trait CheckedIO: BlockAllocation + GenericDiskMethods {
    /// Read a block from the disk, ensuring it has already been allocated, as to not read junk.
    /// Panics if block was not allocated.
    /// 
    /// This should ONLY be used in the cache implementation. If you are dealing with disks directly, you are
    /// doing it wrong.
    fn checked_read(&self, block_number: u16) -> Result<RawBlock, FloppyDriveError> {
        trace!("Performing checked read on block {block_number}...",);
        // Block must be allocated
        assert!(self.is_block_allocated(block_number));
        // This unchecked read is safe, because we've now checked it.
        let result = self.unchecked_read_block(block_number)?;
        trace!("Block read successfully.");
        Ok(result)
    }

    /// Write a block to the disk, ensuring it has not already been allocated, as to not overwrite data.
    ///
    /// Sets the block as used after writing.
    ///
    /// Panics if block was not free.
    fn checked_write(&mut self, block: &RawBlock) -> Result<(), FloppyDriveError> {
        trace!("Performing checked write on block {}...", block.block_origin.block);
        // Make sure block is free
        assert!(!self.is_block_allocated(block.block_origin.block));
        trace!("Block was not already allocated, writing...");
        self.unchecked_write_block(block)?;
        // Now mark the block as allocated.
        trace!("Marking block as allocated...");
        let blocks_allocated = self.allocate_blocks(&[block.block_origin.block].to_vec())?;
        // Make sure it was actually allocated.
        assert_eq!(blocks_allocated, 1);
        // Now decrement the pool header
        trace!("Updating the pool's free block count...");
        trace!("Locking GLOBAL_POOL...");
        GLOBAL_POOL
            .get()
            .expect("single threaded")
            .try_lock()
            .expect("single threaded")
            .header
            .pool_standard_blocks_free -= 1;
        trace!("Block written successfully.");
        Ok(())
    }

    /// Updates an underlying block with new information.
    /// This overwrites the data in the block. (Obviously)
    /// Panics if block was not previously allocated.
    fn checked_update(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        trace!(
            "Performing checked update on block {}...",
            block.block_origin.block
        );
        // Make sure block is allocated already
        assert!(self.is_block_allocated(block.block_origin.block));
        self.unchecked_write_block(block)?;
        trace!("Block updated successfully.");
        Ok(())
    }

    /// Updates several blocks starting at start_block with data. Blocks must already be allocated.
    /// This overwrites the data in the block. (Obviously)
    /// Panics if any of the blocks were not previously allocated.
    fn checked_large_update(&mut self, data: Vec<u8>, start_block: DiskPointer) -> Result<(), BlockError> {
        trace!(
            "Performing checked large update starting on block {}...",
            start_block.block
        );
        // Make sure all of the blocks this refers to are already allocated.
        for block in start_block.block..start_block.block + data.len().div_ceil(512) as u16 {
            assert!(self.is_block_allocated(block));
        }
        self.unchecked_write_large(data, start_block)?;
        trace!("Blocks updated successfully.");
        Ok(())
    }
}
