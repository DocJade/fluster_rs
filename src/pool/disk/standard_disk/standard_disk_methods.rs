// Imports
use std::{
    fs::{File, OpenOptions},
    io::Read,
    u16,
};

use log::error;

use crate::pool::disk::{
    blank_disk::blank_disk_struct::BlankDisk,
    drive_struct::{DiskBootstrap, DiskType, FloppyDriveError},
    generic::{
        block::block_structs::{BlockError, RawBlock},
        disk_trait::GenericDiskMethods,
        io::{read::read_block_direct, write::write_block_direct},
    },
    standard_disk::{
        block::header::header_struct::{StandardDiskHeader, StandardHeaderFlags},
        standard_disk_struct::StandardDisk,
    },
};

// Implementations

// !! Only numbered options should be public! !!

impl DiskBootstrap for StandardDisk {
    fn bootstrap(file: File, disk_number: u16) -> Result<StandardDisk, FloppyDriveError> {
        // Make the disk
        let disk = create(file, disk_number)?;
        // Write the inode block

        // write the directory block
        todo!()
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // TODO: Check CRC.
        todo!()
    }
}

/// Ocasionally, we need to create fake headers during disk loading.
impl StandardDiskHeader {
    fn spoof() -> Self {
        Self {
            flags: StandardHeaderFlags::from_bits_retain(0b11111111),
            disk_number: u16::MAX,
            block_usage_map: [1u8; 360],
        }
    }
}

//
// Public functions
//

/// Initializes a disk by writing header data.
/// Returns the newly created disk.
///
/// This will only work on a disk that is blank / header-less.
/// This will create a disk of any disk number, it is up to the caller to ensure that
/// duplicate disks are not created, and to track the creation of this new disk.
fn create(file: File, disk_number: u16) -> Result<StandardDisk, FloppyDriveError> {
    // Spoof the header, since we're about to give it a new one.
    let mut disk: StandardDisk = StandardDisk {
        number: disk_number,
        header: StandardDiskHeader::spoof(),
        disk_file: file,
    };

    // Now give it some head    er
    // This function checks if the disk is blank for us.
    initialize_numbered(&mut disk, disk_number)?;

    // done
    Ok(disk)
}

//
// Private functions
//

/// Initialize a normal disk for usage. (NOT DATA, NOT POOL)
/// Expects a disk without a header.
/// Will wipe the rest of the disk,
///
/// Errors if provided with a disk that has a header.
// TODO: Somehow prevent duplicate disk numbers?
fn initialize_numbered(disk: &mut StandardDisk, disk_number: u16) -> Result<(), FloppyDriveError> {
    // A new, fresh disk!

    // Time to write in all of the header data.
    // Construct the new header

    // New disks have no flags set.
    let flags: StandardHeaderFlags = StandardHeaderFlags::empty();

    // New disks do have a few pre-allocated blocks, namely the header and the first inode block
    // So construct a map accordingly
    let mut block_usage_map: [u8; 360] = [0u8; 360];
    block_usage_map[0] = 0b11000000; // TODO: Document that the block map is indexed literally, as in block 0 is the first bit.

    let header = StandardDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    };

    // Now serialize that, and write it
    let header_block = &header.to_disk_block();

    // Use the disk interface to write it safely
    disk.write_block(header_block)?;

    // All done!
    Ok(())
}

// Generic disk operations
impl GenericDiskMethods for StandardDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn read_block(self, block_number: u16) -> Result<RawBlock, BlockError> {
        read_block_direct(&self.disk_file, block_number, false)
    }

    #[doc = " Write a block"]
    fn write_block(&mut self, block: &RawBlock) -> Result<(), BlockError> {
        write_block_direct(&self.disk_file, &block)
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
    fn disk_file_mut(&mut self) ->  &mut File {
        &mut self.disk_file
    }
    
    #[doc = " Sync all in-memory information to disk"]
    fn flush(&mut self) -> Result<(), BlockError> {
        self.write_block(&self.header.to_disk_block())
    }
}
