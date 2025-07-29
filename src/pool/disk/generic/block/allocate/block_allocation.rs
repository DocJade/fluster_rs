// Find, reserve, or even free blocks!

// We do not allow these operations to be misused, if invalid state is provided, we panic.
// We will not:
// free bytes that are already free
// allocate bytes that are already allocated
// allocate past the end of the table

use enum_dispatch::enum_dispatch;
use crate::pool::disk::drive_struct::{DiskType, FloppyDriveError};
use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;
use crate::pool::disk::unknown_disk::unknown_disk_struct::UnknownDisk;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use log::{debug, trace};

// To be able to allocate blocks, we need a couple things
#[enum_dispatch(DiskType)]
pub trait BlockAllocation {
    /// Get the block allocation table
    fn get_allocation_table(&self) -> &[u8];

    /// Update and flush the allocation table to disk.
    fn set_allocation_table(&mut self, new_table: &[u8]) -> Result<(), FloppyDriveError>;

    /// Attempts to find free blocks on the disk.
    /// Returns indexes for the found blocks, or returns the number of blocks free if there is not enough space.
    fn find_free_blocks(&self, blocks: u16) -> Result<Vec<u16>, u16> {
        go_find_free_blocks(self, blocks)
    }

    /// Allocates the requested blocks.
    /// Will panic if fed invalid data.
    fn allocate_blocks(&mut self, blocks: &Vec<u16>) -> Result<u16, FloppyDriveError> {
        go_allocate_or_free_blocks(self, blocks, true)
    }

    /// Frees the requested blocks.
    /// Will panic if fed invalid data.
    fn free_blocks(&mut self, blocks: &Vec<u16>) -> Result<u16, FloppyDriveError> {
        go_allocate_or_free_blocks(self, blocks, false)
    }

    /// Check if a specific block is allocated
    fn is_block_allocated(&self, block_number: u16) -> bool {
        go_check_block_allocated(self, block_number)
    }
}

fn go_find_free_blocks<T: BlockAllocation + ?Sized>(
    caller: &T,
    blocks_requested: u16,
) -> Result<Vec<u16>, u16> {
    // The allocation table is a stream of bits, the first bit is the 0th block.

    // Vector of free block locations
    let mut free: Vec<u16> = Vec::new();

    // Now loop through the table looking for free slots.
    for (byte_index, byte) in caller.get_allocation_table().iter().enumerate() {
        // loop over the bits
        for sub_bit in 0..8 {
            // check if the furthest left bit is free.
            // we shift over to the bit we want, then we AND it to check if the highest bit is set.
            // Since we know the bit on one side of the AND is always set, the result will be 0 if the bit is unset.
            // Thus, the result of the if statement will be `0` if the block is free.
            // Could this be done cleaner? Maybe, I'm not very experienced with bitwise operations.
            if (byte << sub_bit) & 0b10000000 == 0 {
                // bit isn't set, the block is free!
                free.push((byte_index as u16 * 8) + sub_bit);

                // Do we have enough blocks now?
                if free.len() == blocks_requested.into() {
                    // Yep!
                    return Ok(free);
                }
            }
        }
    }
    // We've ran out of bytes. We must not have enough free room.
    Err(free.len() as u16)
}

/// allocate false frees the provided bytes.
fn go_allocate_or_free_blocks<T: BlockAllocation + ?Sized>(
    caller: &mut T,
    blocks: &Vec<u16>,
    allocate: bool,
) -> Result<u16, FloppyDriveError> {
    debug!(
        "Attempting to {} {} blocks on the current disk...",
        if allocate { "Allocate" } else { "free" },
        blocks.len()
    );

    // If the user provides a vec with a duplicate item, we will panic from double free / double allocate
    // Vec ordering does not matter, as we calculate the offset from each item
    // The user must allocate/free at least one block, and that block cannot be past the end of the table.
    assert!(*blocks.last().expect("Should allocate at least 1 block.") < 2880);

    // Table to edit
    // 2880 blocks / 8 blocks per bit = 360
    let mut new_allocation_table: [u8; 360] = [0u8; 360];
    new_allocation_table.copy_from_slice(caller.get_allocation_table());

    trace!("Updating blocks...");
    for block in blocks {
        // Get the bit
        // Integer division rounds towards zero, so this is fine.
        let byte: usize = (block / 8) as usize;
        let test_bit: u8 = 0b00000001 << (7 - (block % 8));
        // check the bit
        if new_allocation_table[byte] & test_bit == 0 {
            // block is free.
            if allocate {
                // Good! Send it back
                new_allocation_table[byte] |= test_bit;
                continue;
            } else {
                // We are trying to free a freed block
                panic!("Cannot free block that is already free!")
            }
        } else {
            // Block is not free
            if allocate {
                // Trying to allocate used block.
                panic!("Cannot allocate block that is already allocated!")
            } else {
                // Good! Free the block
                new_allocation_table[byte] ^= test_bit;
                continue;
            }
        }
    }
    trace!("Done updating blocks.");

    // All operations are done, write back the new table
    trace!("Writing back new allocation table...");
    caller.set_allocation_table(&new_allocation_table)?;
    debug!("Done.");
    Ok(blocks.len() as u16)
}

#[inline] // This function should happen inline, since it's such a small operation.
fn go_check_block_allocated<T: BlockAllocation + ?Sized>(caller: &T, block_number: u16) -> bool {
    assert!(block_number < 2880);
    // Integer division rounds towards zero, so this is fine.
    let byte: usize = (block_number / 8) as usize;
    let test_bit: u8 = 0b00000001 << (7 - (block_number % 8));
    // check the bit
    caller.get_allocation_table()[byte] & test_bit != 0
}
