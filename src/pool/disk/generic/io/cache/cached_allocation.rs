// Sidestep the disk if possible when marking a block as allocated.

use std::fs::File;
#[cfg(unix)]
use std::fs::OpenOptions;

use crate::pool::disk::{drive_struct::{DiskBootstrap, DiskType, FloppyDrive, FloppyDriveError, JustDiskType}, generic::{block::{allocate::block_allocation::BlockAllocation, block_structs::RawBlock}, generic_structs::pointer_struct::DiskPointer, io::cache::cache_implementation::{BlockCache, CachedBlock}}, standard_disk::standard_disk_struct::StandardDisk};

pub(super) fn cached_allocation(raw_block: &RawBlock, expected_disk_type: JustDiskType) -> Result<(), FloppyDriveError> {
    // We can create a fake disk to do our allocation against that actually updates the
    // cache instead of the physical disk, assuming the header block is cached, which is very
    // likely.

    // You can only use the allocator on standard disks.
    assert_eq!(expected_disk_type, JustDiskType::Standard);

    // Now, is the header in the cache?
    // The header is block 0 of the disk that this new block wants to allocate
    let header: DiskPointer = DiskPointer {
        disk: raw_block.block_origin.disk,
        block: 0,
    };

    if let Some(header_block) = BlockCache::try_find(header) {
        // Header is cached! We can sneak around the disk access.
        return the_sidestep(raw_block, &header_block)
    }

    // Well, the block was not in the cache, we need to do the allocation normally.
    // Sad.

    let mut disk: StandardDisk = match FloppyDrive::open(header.disk)? {
        DiskType::Standard(standard_disk) => standard_disk,
        _ => panic!("Expected disk was standard, got the unexpected!"),
    };

    // Run the allocation
    let blocks_allocated: u16 = disk.allocate_blocks(&[raw_block.block_origin.block].to_vec())?;

    // make sure we did allocate the block
    assert_eq!(blocks_allocated, 1);

    // All done.
    Ok(())

}

fn the_sidestep(block_to_allocate: &RawBlock, header_block: &CachedBlock) -> Result<(), FloppyDriveError> {
    // We will spoof the disk.
    // This is super risky, maybe,
    // but the speedup and the reduction in disk swapping should be well worth it

    // Hilariously, at the lowest level, flushing a standard disk actually flushes its header to the cache.
    // the entire allocation process never needs to touch the disk file, or the number of the disk. It just
    // needs the block usage map!

    // But for ease of use, we can just extract the entire header from the cached block and construct a disk from that.
    // Luckily I already had a function for this, go figure lmao

    // The allocator never touches the disk file, so we can give it a fake one by just pointing at /dev/null.

    // If you are reading this with the intent of porting to a non-unix platform, first of all:
    // dear god what are you doing

    // Second: you just need to point this at literally any file, since it wont ever actually read or write to it,
    // it just needs a valid handle. Making a tempfile would probably be too slow tho, good luck!

    #[cfg(unix)]
    let spoofed_file: File = OpenOptions::new().read(true).write(true).open("/dev/null").expect("If /dev/null is missing, you have bigger issues.");
    let mut spoofed_disk: StandardDisk = StandardDisk::from_header(header_block.to_raw(), spoofed_file);

    // With our new loaded gun, point it directly at foot.
    let blocks_allocated: u16 = spoofed_disk.allocate_blocks(&[block_to_allocate.block_origin.block].to_vec())?;

    // make sure we did allocate the block
    assert_eq!(blocks_allocated, 1);

    // This flushes to disk for us already. We are done!
    Ok(())
}