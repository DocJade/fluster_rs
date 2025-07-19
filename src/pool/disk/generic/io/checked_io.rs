// IO operations that ensure allocations are properly set.
// We panic in here if we try to read/write in an invalid way, since that indicates a logic error elsewhere.

use crate::pool::disk::generic::{block::{allocate::block_allocation::BlockAllocation, block_structs::{BlockError, RawBlock}}, disk_trait::GenericDiskMethods};

// A fancy new trait thats built out of other traits!
// Automatically add it to all types that implement the subtypes we need.
impl<T: BlockAllocation + GenericDiskMethods> CheckedIO for T {}
pub trait CheckedIO: BlockAllocation + GenericDiskMethods {
    /// Read a block from the disk, ensuring it has already been allocated, as to not read junk.
    /// Panics if block was not allocated.
    fn checked_read(&self, block_number: u16) -> Result<RawBlock, BlockError> {
        // Block must be allocated
        assert!(self.is_block_allocated(block_number));
        self.read_block(block_number)
    }

    /// Write a block to the disk, ensuring it has not already been allocated, as to not overwrite data.
    /// Panics if block was not allocated.
    fn checked_write(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        // Make sure block is free
        assert!(!self.is_block_allocated(block.block_index));
        self.write_block(&block)
    }

    /// Updates an underlying block with new information.
    /// This overwrites the data in the block. (Obviously)
    /// Panics if block was not previously allocated.
    fn checked_update(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        // Make sure block is allocated already
        assert!(self.is_block_allocated(block.block_index));
        self.write_block(&block)
    }
}