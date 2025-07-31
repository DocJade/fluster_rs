// Pool disk

//Imports

use std::fs::File;

use log::error;

use crate::pool::disk::{
    drive_struct::{DiskBootstrap, FloppyDriveError, JustDiskType},
    generic::{
        block::{
            allocate::block_allocation::BlockAllocation,
            block_structs::{BlockError, RawBlock},
            crc::check_crc,
        },
        disk_trait::GenericDiskMethods,
        io::{cache::cache_io::CachedBlockIO, read::read_block_direct, write::write_block_direct},
    },
    pool_disk::block::header::header_struct::PoolDiskHeader,
};

use super::pool_disk_struct::PoolDisk;

// Implementations

impl PoolDisk {
    // todo
}

// Bootstrapping
impl DiskBootstrap for PoolDisk {
    fn bootstrap(_file: File, _disk_number: u16) -> Result<Self, FloppyDriveError> {
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

    fn set_allocation_table(&mut self, new_table: &[u8]) -> Result<(), FloppyDriveError> {
        self.header.block_usage_map = new_table
            .try_into()
            .expect("Incoming table should be the same as outgoing.");
        self.flush()
    }
}

// Generic
impl GenericDiskMethods for PoolDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, block_number: u16) -> Result<RawBlock, BlockError> {
        read_block_direct(&self.disk_file, self.number, block_number, false)
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
    fn flush(&mut self) -> Result<(), FloppyDriveError> {
        CachedBlockIO::update_block(&self.header.to_block(), JustDiskType::Pool)
    }
}
