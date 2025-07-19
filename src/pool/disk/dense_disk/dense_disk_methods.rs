// Sometimes dense people still do things

use std::fs::File;

use crate::pool::disk::{
    drive_struct::{DiskBootstrap, FloppyDriveError},
    generic::{
        block::block_structs::{BlockError, RawBlock},
        disk_trait::GenericDiskMethods,
        io::{read::read_block_direct, write::write_block_direct},
    },
};

use super::dense_disk_struct::DenseDisk;

impl DenseDisk {
    // todo
}

impl DiskBootstrap for DenseDisk {
    fn bootstrap(file: File, disk_number: u16) -> Result<Self, FloppyDriveError> {
        todo!()
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // TODO: Check CRC.
        todo!()
    }
}

impl GenericDiskMethods for DenseDisk {
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
    fn disk_file(self) -> File {
        self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        self.number
    }
    
    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, disk_number: u16) -> () {
        self.number = disk_number;
    }
    
    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) ->  &mut File {
        &mut self.disk_file
    }
    
    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), BlockError> {
        self.write_block(&self.header.to_disk_block())
    }
}
