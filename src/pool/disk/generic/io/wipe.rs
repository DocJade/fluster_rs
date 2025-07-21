// Squeaky clean!

use std::fs::File;

use crate::pool::disk::generic::block::block_structs::{BlockError, RawBlock};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File) -> Result<(), BlockError> {
    // Bye bye!
    for i in 0..2880 {
        wipe_block(disk, i)?;
    }
    Ok(())
}

/// Wipes a single block from a disk
pub(crate) fn wipe_block(disk: &mut File, block_number: u16) -> Result<(), BlockError> {
    // New blank block
    let blanker: RawBlock = RawBlock {
        block_index: block_number,
        data: [0u8; 512],
        originating_disk: None, // This is about to be written.
    };
    super::write::write_block_direct(disk, &blanker)
}
