use std::fs::File;

use crate::pool::disk::{generic::{block::block_structs::{BlockError, RawBlock}, disk_trait::GenericDiskMethods, io::write::write_block_direct}, unknown_disk::unknown_disk_struct::UnknownDisk};



impl GenericDiskMethods for UnknownDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn read_block(self, block_number: u16) -> Result<RawBlock, BlockError> {
        // We cant read from generic disks.
        unreachable!()
    }

    #[doc = " Write a block"]
    fn write_block(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, &block)
    }

    #[doc = " Get the inner file used for IO operations"]
    fn disk_file(&mut self) ->  &mut File {
        &mut self.disk_file
    }

    #[doc = " Get the number of the floppy disk."]
    fn get_disk_number(&self) -> u16 {
        // Unknown disks have no number.
        unreachable!()
    }
}

impl UnknownDisk {
    pub fn new(file: File) -> Self {
        Self {
            disk_file: file
        }
    }
}