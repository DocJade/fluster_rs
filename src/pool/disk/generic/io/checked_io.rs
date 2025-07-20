// IO operations that ensure allocations are properly set.
// We panic in here if we try to read/write in an invalid way, since that indicates a logic error elsewhere.

use log::debug;

use crate::pool::{disk::generic::{block::{allocate::block_allocation::BlockAllocation, block_structs::{BlockError, RawBlock}}, disk_trait::GenericDiskMethods}, pool_struct::GLOBAL_POOL};

// A fancy new trait thats built out of other traits!
// Automatically add it to all types that implement the subtypes we need.
impl<T: BlockAllocation + GenericDiskMethods> CheckedIO for T {}
pub trait CheckedIO: BlockAllocation + GenericDiskMethods {
    /// Read a block from the disk, ensuring it has already been allocated, as to not read junk.
    /// Panics if block was not allocated.
    fn checked_read(&self, block_number: u16) -> Result<RawBlock, BlockError> {
        debug!("Performing checked read on block {block_number}...",);
        // Block must be allocated
        assert!(self.is_block_allocated(block_number));
        let result = self.read_block(block_number)?;
        debug!("Block read successfully.");
        Ok(result)
    }

    /// Write a block to the disk, ensuring it has not already been allocated, as to not overwrite data.
    /// 
    /// Sets the block as used after writing.
    /// 
    /// Panics if block was not free.
    fn checked_write(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        debug!("Performing checked write on block {}...", block.block_index);
        // Make sure block is free
        assert!(!self.is_block_allocated(block.block_index));
        self.write_block(block)?;
        // Now mark the block as allocated.
        let blocks_allocated = self.allocate_blocks(&[block.block_index].to_vec());
        // Make sure it was actually allocated.
        assert_eq!(blocks_allocated, 1);
        // Now decrement the pool header
        debug!("Updating the pool's free block count...");
        debug!("Locking GLOBAL_POOL...");
        GLOBAL_POOL.get().expect("single threaded").try_lock().expect("single threaded").header.pool_standard_blocks_free -= 1;
        debug!("Block written successfully.");
        Ok(())
    }
    
    /// Updates an underlying block with new information.
    /// This overwrites the data in the block. (Obviously)
    /// Panics if block was not previously allocated.
    fn checked_update(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        debug!("Performing checked update on block {}...", block.block_index);
        // Make sure block is allocated already
        assert!(self.is_block_allocated(block.block_index));
        self.write_block(&block)?;
        debug!("Block updated successfully.");
        Ok(())
    }
}