// Imports
use std::{
    fs::{File, OpenOptions}, io::Read, u16
};

use log::{debug, error};

use crate::pool::{disk::{
    blank_disk::blank_disk_struct::BlankDisk,
    drive_struct::{DiskBootstrap, DiskType, FloppyDriveError},
    generic::{
        block::{allocate::block_allocation::BlockAllocation, block_structs::{BlockError, RawBlock}}, disk_trait::GenericDiskMethods, generic_structs::pointer_struct::DiskPointer, io::{checked_io::CheckedIO, read::read_block_direct, write::write_block_direct}
    },
    standard_disk::{
        block::{directory::directory_struct::DirectoryBlock, header::header_struct::{StandardDiskHeader, StandardHeaderFlags}, inode::inode_struct::{Inode, InodeBlock, InodeDirectory, InodeFlags, InodeTimestamp}},
        standard_disk_struct::StandardDisk,
    },
}, pool_actions::pool_struct::{Pool, GLOBAL_POOL}};


// Implementations

// !! Only numbered options should be public! !!

impl DiskBootstrap for StandardDisk {
    fn bootstrap(file: File, disk_number: u16) -> Result<StandardDisk, FloppyDriveError> {
        debug!("Boostrapping a standard disk...");
        
        // Update how many blocks are free in the pool
        // New standard disks have only the header allocated.
        // 2880 - 1 = 2879
        // But, the disk setup process will automatically decrement this count for us.
        debug!("Locking GLOBAL_POOL...");
        GLOBAL_POOL.get().expect("single threaded").try_lock().expect("single threaded").header.pool_standard_blocks_free += 2880;
        
        // Make the disk
        debug!("Running create...");
        let mut disk = create(file, disk_number)?;
        // Now that we have a disk, we can use the safe IO.
        
        
        
        // Write the inode block
        debug!("Writing inode block...");
        let inode_block = InodeBlock::new();
        let inode_writer = inode_block.to_block(1);
        disk.checked_write(&inode_writer)?;

        //TODO:Testing
        assert!(disk.is_block_allocated(1));
        
        // if this is disk 1 then we need to add:
        // Directory block
        // the root directory to that block
        if disk_number != 1 {
            // Dont need to do anything.
            debug!("Done bootstrapping standard disk.");
            return Ok(disk)
        }
        debug!("This is the origin standard disk, doing a bit more...");
        
        // Create the directory block
        let directory_block: DirectoryBlock = DirectoryBlock::new();
        
        // Write that to the disk. It goes in block 2.
        debug!("Writing root directory block...");
        disk.checked_write(&directory_block.to_block(2))?;
        
        // Now we need to manually add the inode that points to it. Because the inode at the 0 index
        // of block 1 is the inode that points to the root directory
        
        // Add the root inode
        let pointer_to_dat_mf: DiskPointer = DiskPointer {
            disk: 1,
            block: 2, // The root directory is at block 2.
        };
        
        let root_directory_inode = InodeDirectory::from_disk_pointer(pointer_to_dat_mf);
        let right_now = InodeTimestamp::now();
        let the_actual_inode: Inode = Inode {
            flags: InodeFlags::MarkerBit, // Not a file, so only the marker.
            file: None,
            directory: Some(root_directory_inode),
            created: right_now,
            modified: right_now,
        };
        
        debug!("Writing root directory inode...");
        let inode_result = Pool::add_inode(the_actual_inode).expect("We should have room.");
        // Make sure that actually ended up at the right spot.
        assert_eq!(inode_result.disk, Some(1));
        assert_eq!(inode_result.block, 1);
        assert_eq!(inode_result.offset, 0);
        
        // All done!
        debug!("Done bootstrapping standard disk.");
        Ok(disk)
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // load in the header
        let header: StandardDiskHeader = StandardDiskHeader::from_block(&block).expect("Already checked type.");
        StandardDisk {
            number: header.disk_number,
            disk_file: file,
            header,
        }
    }
}

// This disk has block level allocations
impl BlockAllocation for StandardDisk {
    fn get_allocation_table(&self) -> &[u8] {
        &self.header.block_usage_map
    }

    fn set_allocation_table(&mut self, new_table: &[u8]) -> Result<(), BlockError> {
        self.header.block_usage_map = new_table.try_into().expect("Incoming table should be the same as outgoing.");
        self.flush()
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
    debug!("Creating new standard disk {disk_number}");
    debug!("Creating spoofed disk...");
    // Spoof the header, since we're about to give it a new one.
    let mut disk: StandardDisk = StandardDisk {
        number: disk_number,
        header: StandardDiskHeader::spoof(),
        disk_file: file,
    };
    
    // Now give it some head    er
    // This function checks if the disk is blank for us.
    debug!("Initializing the disk from spoof...");
    initialize_numbered(&mut disk, disk_number)?;
    
    // done
    debug!("Done creating disk.");
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
    debug!("Initializing a new standard disk...");
    // A new, fresh disk!
    
    // Time to write in all of the header data.
    // Construct the new header
    
    // New disks have no flags set, besides the required marker bit
    let mut flags: StandardHeaderFlags = StandardHeaderFlags::empty();
    flags.insert(StandardHeaderFlags::Marker);
    
    // New disks do have a few pre-allocated blocks, namely the header and the first inode block
    // But they will be allocated during the creation process.
    let block_usage_map: [u8; 360] = [0u8; 360];
    
    let header = StandardDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    };

    
    
    // Now serialize that, and write it
    let header_block = &header.to_block();

    // Update the header on the provided disk, since it's currently spoofed.
    disk.header = header;
    
    // Use the disk interface to write it safely
    // This will allocate the header block
    debug!("Writing header...");
    disk.checked_write(header_block)?;
    debug!("Header written.");
    
    // All done!
    debug!("Done initializing.");
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
    #[doc = " Headers and such."]
    fn flush(&mut self) -> Result<(),BlockError> {
        // We need to write the header back to disk, since that is the only
        // information we can edit in memory without immediately writing.
        self.checked_update(&self.header.to_block())
    }
}