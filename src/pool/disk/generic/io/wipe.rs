// Squeaky clean!

use std::fs::File;

use crate::pool::disk::generic::{block::block_structs::{BlockError, RawBlock}, generic_structs::pointer_struct::DiskPointer, io::cache::cache_io::CachedBlockIO};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File, disk_number: u16) -> Result<(), BlockError> {
    // Bye bye!
    for block in 0..2880 {
        let pointer: DiskPointer = DiskPointer {
            block,
            disk: disk_number,
        };
        wipe_block(disk, pointer)?;
    }
    Ok(())
}

/// Wipes a single block from a disk
pub(crate) fn wipe_block(disk: &mut File, block_origin: DiskPointer) -> Result<(), BlockError> {
    // New blank block
    let blanker: RawBlock = RawBlock {
        data: [0u8; 512],
        block_origin,
    };
    super::write::write_block_direct(disk, &blanker)?;
    // We also need to update the cache to remove this block
    CachedBlockIO::remove_block(&block_origin);
    Ok(())
}
