use std::fs::File;

use crate::pool::disk::{
    generic::{
        block::block_structs::{BlockError, RawBlock},
        disk_trait::GenericDiskMethods,
        io::write::write_block_direct,
    },
    unknown_disk::unknown_disk_struct::UnknownDisk,
};

impl GenericDiskMethods for UnknownDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, _block_number: u16) -> Result<RawBlock, BlockError> {
        // We cant read from generic disks.
        unreachable!()
    }

    #[doc = " Write a block"]
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, &block)
    }

    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(self) -> File {
        self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        // Unknown disks have no number.
        unreachable!()
    }
    
    #[doc = " Set the number of this disk."]
    fn set_disk_number(&mut self, _disk_number:u16) -> () {
        // You cannot set the disk number of an unknown disk.
        unreachable!()
    }
    
    #[doc = " Get the inner file used for write operations"]
    fn disk_file_mut(&mut self) ->  &mut File {
        &mut self.disk_file
    }
    
    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), BlockError> {
        // There is no in-memory information for this disk.
        unreachable!()
    }
}

impl UnknownDisk {
    pub fn new(file: File) -> Self {
        Self { disk_file: file }
    }
}
