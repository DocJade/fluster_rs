// Yep.

use std::fs::File;

use crate::pool::disk::{blank_disk::blank_disk_struct::BlankDisk, generic::{block::block_structs::{BlockError, RawBlock}, disk_trait::GenericDiskMethods, io::write::write_block_direct}};

impl GenericDiskMethods for BlankDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn read_block(self, block_number: u16) -> Result<RawBlock, BlockError> {
        // We should NEVER read a block from a blank disk, why would we do that?
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
        // Why are we getting the disk number of a blank floppy?
        unreachable!()
    }
}

// Occasionally we need a new blank disk
impl BlankDisk {
    pub fn new(file: File) -> Self {
        Self {
            disk_file: file
        }
    }
}