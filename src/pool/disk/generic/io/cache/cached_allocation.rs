// Sidestep the disk if possible when marking a block as allocated.

use std::process::exit;

use log::error;

use crate::{error_types::drive::{DriveError, DriveIOError}, pool::disk::{
    generic::{
        block::{
            allocate::block_allocation::BlockAllocation,
            block_structs::RawBlock
        },
        generic_structs::pointer_struct::DiskPointer,
        io::cache::cache_io::CachedBlockIO
    },
    standard_disk::block::header::header_struct::StandardDiskHeader
}};

// To not require a rewrite of pool block allocation logic, we will make fake disks for it to use.
pub(crate) struct CachedAllocationDisk {
    /// The header of the disk we are imitating
    imitated_header: StandardDiskHeader
}

impl CachedAllocationDisk {
    /// Attempt to create a new cached disk for allocation.
    /// 
    /// To flush the new allocation table to the cache, this needs to be dropped.
    /// Thus, if you allocate then immediately write, you need to drop this before the write.
    pub(crate) fn open(disk_number: u16) -> Result<Self, DriveError> {
        // Go get the header for this disk. Usually this is cached, but
        // will fall through if needed.
        let header_pointer: DiskPointer = DiskPointer {
            disk: disk_number,
            block: 0,
        };

        // We will attempt to read the 

        let read: RawBlock = CachedBlockIO::read_block(header_pointer)?;
        let imitated_header: StandardDiskHeader = StandardDiskHeader::from_block(&read);
        Ok(
            Self {
            imitated_header
            }
        )
    }
}

// We need to support all of the allocation methods that disks normally use.

impl BlockAllocation for CachedAllocationDisk {
    #[doc = " Get the block allocation table"]
    fn get_allocation_table(&self) -> &[u8] {
        &self.imitated_header.block_usage_map
    }

    #[doc = " Update and flush the allocation table to disk."]
    fn set_allocation_table(&mut self,new_table: &[u8]) -> Result<(), DriveIOError> {
        self.imitated_header.block_usage_map = new_table
            .try_into()
            .expect("Incoming table should be the same as outgoing.");
        Ok(())
    }
}

// When these fake disks are dropped, their updated (if updated) blocks need to go into the cache
impl Drop for CachedAllocationDisk {
    fn drop(&mut self) {
        // Put our fake header in the cache.
        let updated = self.imitated_header.to_block();
        // If this fails we are major cooked, we will try 10 times.
        for i in (1..=10).rev() {
            let result = CachedBlockIO::update_block(&updated);
            if let Err(bad) = result {
                // UH OH
                error!("Attempting to flush a CachedAllocationDisk is failing!");
                error!("{bad:#?}");
                // If we are out of attempts, we must die.
                let remaining_attempts = i - 1;
                if remaining_attempts == 0 {
                    // Cooked.
                    error!("Well.. Shit!");
                    error!("Filesystem is in an unrecoverable state!");
                    error!("Giving up.");
                    exit(1) // bye bye!
                }
                error!("{remaining_attempts} attempts remaining!")
            } else {
                // Worked! All done.
                break
            }
        }
        // Block has been put in the cache.
    }
}