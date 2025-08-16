// Squeaky clean!

use std::{fs::File, ops::Rem};

use crate::pool::disk::generic::{
    block::block_structs::RawBlock,
    generic_structs::pointer_struct::DiskPointer
};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File) -> Result<(), BlockError> {
    // Bye bye!
    for block_number in 0..2880_u16 {
        wipe_block(disk, block_number)?;
    }
    Ok(())
}

/// Wipes a single block from a disk
pub(crate) fn wipe_block(disk: &mut File, block_number: u16) -> Result<(), BlockError> {
    // New blank block.
    // We can use a fake disk number, since we are directly writing.
    let block_origin = DiskPointer {
        disk: 21,
        block: block_number,
    };

    let blanker: RawBlock = RawBlock {
        data: [0u8; 512],
        block_origin,
    };
    super::write::write_block_direct(disk, &blanker)?;
    // If this is being ran during runtime, we should be clearing out these blocks from the cache,
    // but if you are destroying a disk, this should NOT be a disk that currently exists in the pool, since
    // this should only be called on new disks. Thus we do not touch the cache here.

    // Print progress information.
    // 10% increments.
    let descriptive_variable_name = block_number + 1;
    if descriptive_variable_name.rem(288) == 0 {
        println!("{}%...", descriptive_variable_name.div_ceil(288))
    }

    Ok(())
}
