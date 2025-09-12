// Yep.

use std::fs::File;

use log::error;

use crate::{
    error_types::drive::DriveError,
        pool::disk::{
        blank_disk::blank_disk_struct::BlankDisk,
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock
            },
            disk_trait::GenericDiskMethods,
            generic_structs::pointer_struct::DiskPointer,
            io::write::write_block_direct
        }
    }
};

impl GenericDiskMethods for BlankDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, _block_number: u16) -> Result<RawBlock, DriveError> {
        // We should NEVER read a block from a blank disk, why would we do that?
        unreachable!("Attempted to read a block from a blank disk! Not allowed! You need to turn it into another type first!")
    }

    #[doc = " Write a block"]
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), DriveError> {
        write_block_direct(&self.disk_file, block)
    }

    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(self) -> File {
        self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        // Why are we getting the disk number of a blank floppy?
        error!("Attempted to get the disk number of a blank disk! Not allowed!");
        // We will ignore the action and return a nonsensical number, this prevents fluster
        // from crashing if you have a disk blank disk in the drive after finishing troubleshooting.
        u16::MAX
    }

    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, _disk_number: u16) -> () {
        // You cannot set the number of a blank disk.
        // Trying to set the disk number is doomed to fail, because at this point it thinks its an initialized disk, which it is not.
        unreachable!("Attempted to set the disk number of a blank disk! Not allowed!")
    }

    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) -> &mut File {
        &mut self.disk_file
    }

    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), DriveError> {
        // There is no in-memory information for this disk.
        // So we can safely ignore this.
        Ok(())
    }
    
    #[doc = " Write chunked data, starting at a block."]
    fn unchecked_write_large(&mut self, data:Vec<u8>, start_block:DiskPointer) -> Result<(), DriveError> {
        crate::pool::disk::generic::io::write::write_large_direct(&self.disk_file, &data, start_block)
    }
    
    #[doc = " Read multiple blocks"]
    #[doc = " Does not check CRC!"]
    fn unchecked_read_multiple_blocks(&self, _block_number: u16, _num_block_to_read: u16) -> Result<Vec<RawBlock>,DriveError> {
        unreachable!("Attempted to read a block from a blank disk! Not allowed! You need to turn it into another type first!")
    }
}

// Occasionally we need a new blank disk
impl BlankDisk {
    pub fn new(file: File) -> Self {
        Self { disk_file: file }
    }
}

impl BlockAllocation for BlankDisk {
    #[doc = " Get the block allocation table"]
    fn get_allocation_table(&self) ->  &[u8] {
        unreachable!("Block allocation is not supported on blank disks.")
    }
    
    #[doc = " Update and flush the allocation table to disk."]
    fn set_allocation_table(&mut self, _new_table: &[u8]) -> Result<(), DriveError> {
        unreachable!("Block allocation is not supported on blank disks.")
        
    }
}