// Pool disk

//Imports

use std::fs::File;

use log::error;

use crate::pool::disk::{drive_struct::{DiskBootstrap, FloppyDriveError}, generic::{block::{block_structs::{BlockError, RawBlock}, crc::check_crc}, disk_trait::GenericDiskMethods, io::{read::read_block_direct, write::write_block_direct}}, pool_disk::block::header::header_struct::PoolDiskHeader};

use super::pool_disk_struct::PoolDisk;

// Implementations

impl PoolDisk {
    // todo
}

// Bootstrapping
impl DiskBootstrap for PoolDisk {
    fn bootstrap(file: std::fs::File, disk_number: u16) -> Result<Self, FloppyDriveError> {
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
        Self {
            number: 0,
            #[allow(clippy::unwrap_used)] // TODO: remove unwrap.
            header: PoolDiskHeader::from_block(&block).unwrap(), 
            disk_file: file,
        }
    }
}

// Generic
impl GenericDiskMethods for PoolDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn read_block(self, block_number: u16) -> Result<RawBlock, BlockError> {
        read_block_direct(&self.disk_file, block_number, false)
    }

    #[doc = " Write a block"]
    fn write_block(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, block)
    }

    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(&mut self) ->  &mut File {
        &mut self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        self.number
    }
}