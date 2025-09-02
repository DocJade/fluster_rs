// Imports
use std::fs::File;

use log::debug;

use crate::{error_types::drive::DriveError, pool::{
    disk::{
        drive_struct::{
            DiskBootstrap,
            DiskType,
            FloppyDrive
        },
        generic::{
            block::{
                allocate::block_allocation::BlockAllocation,
                block_structs::RawBlock,
            },
            disk_trait::GenericDiskMethods,
            generic_structs::pointer_struct::DiskPointer,
            io::{
                cache::cache_io::CachedBlockIO,
                read::{read_block_direct, read_multiple_blocks_direct},
                write::{
                    write_block_direct,
                    write_large_direct
                }
            },
        },
        standard_disk::{
            block::{
                directory::directory_struct::DirectoryBlock,
                header::header_struct::{
                    StandardDiskHeader,
                    StandardHeaderFlags
                },
                inode::inode_struct::{
                    Inode,
                    InodeBlock,
                    InodeDirectory,
                    InodeFlags,
                    InodeTimestamp,
                },
            },
            standard_disk_struct::StandardDisk,
        },
    },
    pool_actions::pool_struct::{
        Pool,
        GLOBAL_POOL
    },
}};

// Implementations

// !! Only numbered options should be public! !!

impl DiskBootstrap for StandardDisk {
    fn bootstrap(file: File, disk_number: u16) -> Result<StandardDisk, DriveError> {
        debug!("Boostrapping a standard disk...");

        // Update how many blocks are free in the pool
        // New standard disks have only the header allocated.
        // 2880 - 1 = 2879
        // But, the disk setup process will automatically decrement this count for us.
        debug!("Locking GLOBAL_POOL...");
        GLOBAL_POOL
            .get()
            .expect("single threaded")
            .try_lock()
            .expect("single threaded")
            .header
            .pool_standard_blocks_free += 2880;

        // Make the disk
        debug!("Running create...");
        let mut disk = create(file, disk_number)?;
        // Now that we have a disk, we can use the safe IO.

        // if this is disk 1 then we need to add:
        // Inode block
        // Directory block
        // the root directory to that block
        if disk_number != 1 {
            // Dont need to do anything.
            debug!("Done bootstrapping standard disk.");
            return Ok(disk);
        }
        debug!("This is the origin standard disk, doing a bit more...");

        // the new origin disk needs to have blocks 1 and 2 allocated as well for the first inode/directory blocks.
        let _ = disk.allocate_blocks(&vec![1,2])?;
        // Ignoring resulting value, since it will always be 2.
        // Which means we also need to update the pool block count again.


        // Write the inode block
        debug!("Writing inode block...");
        let inode_block_origin: DiskPointer = DiskPointer {
            disk: disk_number,
            block: 1,
        };
        let inode_block = InodeBlock::new(inode_block_origin);
        let inode_writer = inode_block.to_block();
        disk.unchecked_write_block(&inode_writer)?;
        
        // Create the directory block
        
        // Write that to the disk. It goes in block 2.
        debug!("Writing root directory block...");
        let directory_block_origin: DiskPointer = DiskPointer {
            disk: disk_number,
            block: 2,
        };
        let directory_block: DirectoryBlock = DirectoryBlock::new(directory_block_origin);
        let the_directory_block: RawBlock = directory_block.to_block();
        disk.unchecked_write_block(&the_directory_block)?;
        
        // Now we need to manually add the inode that points to it. Because the inode at the 0 index
        // of block 1 is the inode that points to the root directory
        
        // Add the root inode
        debug!("Writing root directory inode...");
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

        let inode_result = Pool::add_inode(the_actual_inode).expect("We should have room.");
        // Make sure that actually ended up at the right spot.
        assert_eq!(inode_result.pointer, inode_block_origin);
        assert_eq!(inode_result.offset, 0);

        // All done!
        debug!("Done bootstrapping standard disk.");
        // Since we wrote information to it, we need to read in that disk again before returning it
        #[allow(deprecated)] // We do not use the cache while bootstrapping.
        let finished_disk: StandardDisk = match FloppyDrive::open(disk_number)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("I would eat my shoes if this happened."),
        };
        Ok(finished_disk)
    }

    fn from_header(block: RawBlock, file: File) -> Self {
        // load in the header.
        // We assume the caller has passed in the freshest version of the header.
        let header: StandardDiskHeader =
            StandardDiskHeader::from_block(&block);
            
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

    fn set_allocation_table(&mut self, new_table: &[u8]) -> Result<(), DriveError> {
        self.header.block_usage_map = new_table
            .try_into()
            .expect("Incoming table should be the same as outgoing.");
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
fn create(file: File, disk_number: u16) -> Result<StandardDisk, DriveError> {
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
fn initialize_numbered(disk: &mut StandardDisk, disk_number: u16) -> Result<(), DriveError> {
    debug!("Initializing a new standard disk...");
    // A new, fresh disk!

    // Time to write in all of the header data.
    // Construct the new header

    // New disks have no flags set, besides the required marker bit
    let mut flags: StandardHeaderFlags = StandardHeaderFlags::empty();
    flags.insert(StandardHeaderFlags::Marker);

    // New disks do have a few pre-allocated blocks, namely the header and the first inode block
    // But they will be allocated during the creation process.
    let mut block_usage_map: [u8; 360] = [0u8; 360];
    // We must mark the first block used, because we cant run the allocated without being able to update
    // the header block (which needs to be allocated for the update)
    block_usage_map[0] = 0b10000000;

    let header = StandardDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    };

    // Now serialize that, and write it
    let header_block = &header.to_block();

    // Update the header on the provided disk, since it's currently spoofed.
    disk.header = header;

    // Since this is a brand new disk without proper header information finalized, we have to do a direct write here

    debug!("Writing header...");
    disk.unchecked_write_block(header_block)?;
    debug!("Header written.");

    // All done!
    debug!("Done initializing.");
    Ok(())
}

// Generic disk operations
impl GenericDiskMethods for StandardDisk {
    #[doc = " Read a block"]
    #[doc = " Cannot bypass CRC."]
    fn unchecked_read_block(&self, block_number: u16) -> Result<RawBlock, DriveError> {
        read_block_direct(&self.disk_file, self.number, block_number, false)
    }

    #[doc = " Write a block"]
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), DriveError> {
        write_block_direct(&self.disk_file, block)
    }

    #[doc = " Write chunked data, starting at a block."]
    fn unchecked_write_large(&mut self, data:Vec<u8>, start_block:DiskPointer) -> Result<(), DriveError> {
        write_large_direct(&self.disk_file, &data, start_block)
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
    #[doc = " Headers and such."]
    fn flush(&mut self) -> Result<(), DriveError> {
        // Not really to disk, but if nobody is looking...
        CachedBlockIO::update_block(&self.header.to_block())
    }
    
    #[doc = " Read multiple blocks"]
    #[doc = " Does not check CRC!"]
    fn unchecked_read_multiple_blocks(&self, block_number: u16, num_block_to_read: u16) -> Result<Vec<RawBlock>, DriveError> {
        read_multiple_blocks_direct(&self.disk_file, self.number, block_number, num_block_to_read)
    }

}
