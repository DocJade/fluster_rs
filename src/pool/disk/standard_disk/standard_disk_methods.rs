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
        block::{allocate::block_allocation::BlockAllocation, block_structs::{BlockError, RawBlock}},
        disk_trait::GenericDiskMethods,
        io::{checked_io::CheckedIO, read::read_block_direct, write::write_block_direct},
    },
    standard_disk::{
        block::{directory::directory_struct::DirectoryBlock, header::header_struct::{StandardDiskHeader, StandardHeaderFlags}, inode::inode_struct::InodeBlock},
        standard_disk_struct::StandardDisk,
    },
};

// Implementations

// !! Only numbered options should be public! !!

impl DiskBootstrap for StandardDisk {
    fn bootstrap(file: File, disk_number: u16) -> Result<StandardDisk, FloppyDriveError> {
        // Make the disk
        let mut disk = create(file, disk_number)?;
        // Now that we have a disk, we can use the safe IO.

        // Write the inode block
        let inode_block = InodeBlock::new();
        let inode_writer = inode_block.to_block(1);
        disk.checked_write(&inode_writer)?;

        // write the directory block
        let directory_block = DirectoryBlock::new();
        let directory_writer = directory_block.to_block(2);
        disk.checked_write(&directory_writer)?;

        // if this is disk 1 then we need to add the root directory and inode
        if disk_number == 1 {
            // The special case!
            todo!()
        }
        todo!()
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // TODO: Check CRC.
        todo!()
    }
}

// This disk has block level allocations
impl BlockAllocation for StandardDisk {
    fn get_allocation_table(&self) -> &[u8] {
        &self.block_usage_map
    }

    fn set_allocation_table(&mut self, new_table: &[u8]) {
        self.block_usage_map = new_table.try_into().expect("Incoming table should be the same as outgoing.");
    }
}

/// Ocasionally, we need to create fake headers during disk loading.
impl StandardDiskHeader {
    fn spoof() -> Self {
        Self {
            flags: StandardHeaderFlags::from_bits_retain(0b00100000), // Gotta set that marker bit.
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
        block_usage_map: [0u8; 360],
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
    block_usage_map[0] = 0b10000000; // We will set up the other 2 blocks later

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
    fn read_block(&self, block_number: u16) -> Result<RawBlock, BlockError> {
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
