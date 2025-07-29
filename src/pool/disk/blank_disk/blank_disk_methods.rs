// Yep.

use std::fs::File;

use crate::pool::disk::{
    blank_disk::blank_disk_struct::BlankDisk, drive_struct::FloppyDriveError, generic::{
        block::{allocate::block_allocation::BlockAllocation, block_structs::{BlockError, RawBlock}},
        disk_trait::GenericDiskMethods,
        io::write::write_block_direct,
    }
};

impl GenericDiskMethods for BlankDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, _block_number: u16) -> Result<RawBlock, BlockError> {
        // We should NEVER read a block from a blank disk, why would we do that?
        unreachable!()
    }

    #[doc = " Write a block"]
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, block)
    }

    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(self) -> File {
        self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        // Why are we getting the disk number of a blank floppy?
        unreachable!()
    }

    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, _disk_number: u16) -> () {
        // You cannot set the number of a blank disk.
        unreachable!()
    }

    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) -> &mut File {
        &mut self.disk_file
    }

    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), FloppyDriveError> {
        // There is no in-memory information for this disk.
        unreachable!()
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
        panic!("Why are we allocating blocks on a blank disk?")
    }
    
    #[doc = " Update and flush the allocation table to disk."]
    fn set_allocation_table(&mut self, _new_table: &[u8]) -> Result<(), FloppyDriveError> {
        panic!("Why are we allocating blocks on a blank disk?")
        
    }
}