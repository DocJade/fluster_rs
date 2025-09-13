use std::fs::File;

use crate::{
    error_types::drive::DriveError,
    pool::disk::{
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock
            },
            disk_trait::GenericDiskMethods,
            generic_structs::pointer_struct::DiskPointer,
            io::write::write_block_direct
        },
        unknown_disk::unknown_disk_struct::UnknownDisk
    }
};

impl GenericDiskMethods for UnknownDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, _block_number: u16) -> Result<RawBlock, DriveError> {
        // We cant read from generic disks.
        panic!("Attempted to read blocks from a disk we know nothing about, we cannot do that.");
    }
    
    #[doc = " Write a block"]
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), DriveError> {
        // This is the first call, we have not recursed.
        write_block_direct(&self.disk_file, block, false)
    }
    
    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(self) -> File {
        self.disk_file
    }
    
    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        // Unknown disks have no number.
        panic!("Attempted to get the disk number of a disk we know nothing about! Not allowed!");
    }
    
    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, _disk_number: u16) {
        // You cannot set the disk number of an unknown disk.
        panic!("Attempted to set the disk number of a disk we know nothing about! Not allowed!");

    }

    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) -> &mut File {
        &mut self.disk_file
    }

    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), DriveError> {
        // There is no in-memory information for this disk.
        // So just don't do anything.
        Ok(())
    }
    
    #[doc = " Write chunked data, starting at a block."]
    fn unchecked_write_large(&mut self, _data: Vec<u8>, _start_block: DiskPointer) -> Result<(), DriveError> {
        panic!("Cannot do large writes to unknown disks!");
    }
    
    #[doc = " Read multiple blocks"]
    #[doc = " Does not check CRC!"]
    fn unchecked_read_multiple_blocks(&self, _block_number: u16, _num_block_to_read: u16) -> Result<Vec<RawBlock>,DriveError> {
        panic!("Cannot do large reads from unknown disks!");
    }
}

impl UnknownDisk {
    pub fn new(file: File) -> Self {
        Self { disk_file: file }
    }
}

impl BlockAllocation for UnknownDisk {
    #[doc = " Get the block allocation table"]
    fn get_allocation_table(&self) ->  &[u8] {
        panic!("Unknown disks do not support allocations.");

    }
    
    #[doc = " Update and flush the allocation table to disk."]
    fn set_allocation_table(&mut self, _new_table: &[u8]) -> Result<(), DriveError> {
        panic!("Unknown disks do not support allocations.");

    }
}