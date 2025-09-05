// Pool disk

//Imports

use std::fs::File;

use log::error;

use crate::{
    error_types::drive::DriveError,
    pool::disk::{
        drive_struct::DiskBootstrap,
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock,
                crc::check_crc,
            }, disk_trait::GenericDiskMethods, generic_structs::pointer_struct::DiskPointer, io::{read::read_block_direct, write::write_block_direct}
        },
        pool_disk::block::header::header_struct::PoolDiskHeader,
    }
};

use super::pool_disk_struct::PoolDisk;

// Implementations

impl PoolDisk {
    // todo
}

// Bootstrapping
impl DiskBootstrap for PoolDisk {
    fn bootstrap(_file: File, _disk_number: u16) -> Result<Self, DriveError> {
        // Annoyingly, we do bootstrapping of the pool disk from elsewhere, so this has to be here just
        // to fill criteria for DiskBootstrap
        todo!()
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // Immediately check the CRC of the incoming block, we don't know what state it's in
        if !check_crc(block.data) {
            // CRC failed!
            error!("Someday we should be able to recover from crc checks... that is not today.");
            todo!()
        };
        // CRC is good, construct the disk...
        #[allow(clippy::unwrap_used)] // TODO: remove unwrap.
        let header = PoolDiskHeader::from_block(&block).unwrap();
        Self {
            number: 0, // The pool disk is always disk 0
            header,
            disk_file: file,
        }
    }
}

// Block allocator
// This disk has block level allocations
impl BlockAllocation for PoolDisk {
    fn get_allocation_table(&self) -> &[u8] {
        &self.header.block_usage_map
    }

    fn set_allocation_table(&mut self, new_table: &[u8]) -> Result<(), DriveError> {
        self.header.block_usage_map = new_table
            .try_into()
            .expect("Incoming table size should be the same as outgoing.");
        self.flush()
    }
}

// Generic
impl GenericDiskMethods for PoolDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, block_number: u16) -> Result<RawBlock, DriveError> {
        read_block_direct(&self.disk_file, self.number, block_number, false)
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
        self.number
    }

    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, disk_number: u16) {
        self.number = disk_number
    }

    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) -> &mut File {
        &mut self.disk_file
    }

    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), DriveError> {
        error!("You cannot call flush on a pool disk header.");
        error!("This must be handled manually via a disk unchecked write.");
        panic!("Tried to flush a pool header with .flush() !");
    }
    
    #[doc = " Write chunked data, starting at a block."]
    fn unchecked_write_large(&mut self, _data:Vec<u8>, _start_block: DiskPointer) -> Result<(), DriveError> {
        // We do not allow large writes to the pool disk.
        // Man the pool disk really ended up useless didn't it?
        panic!("No large writes on pool disks.");
    }
    
    #[doc = " Read multiple blocks"]
    #[doc = " Does not check CRC!"]
    fn unchecked_read_multiple_blocks(&self, _block_number: u16, _num_block_to_read: u16) -> Result<Vec<RawBlock>, DriveError> {
        panic!("No large reads on pool disks.");
    }
}
