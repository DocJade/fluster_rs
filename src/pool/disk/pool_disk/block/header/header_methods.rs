// So no head?

// Imports

use std::process::exit;

use log::debug;

use crate::error_types::drive::DriveError;
use crate::error_types::header::HeaderError;
use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::drive_methods::check_for_magic;
use crate::pool::disk::drive_methods::display_info_and_ask_wipe;
use crate::pool::disk::drive_struct::DiskType;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::generic::block::crc::add_crc_to_block;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::generic::io::wipe::destroy_disk;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolHeaderFlags;
use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;
use super::header_struct::PoolDiskHeader;

// Implementations

impl PoolDiskHeader {
    /// Reterive the header from the pool disk
    pub fn read() -> Result<Self, DriveError> {
        read_pool_header_from_disk()
    }
    /// Overwrite the current header stored on the pool disk.
    pub fn write(&self) -> Result<(), DriveError> {
        write_pool_header_to_disk(self)
    }
    /// Convert the pool header into a RawBlock
    pub fn to_block(self) -> RawBlock {
        pool_header_to_raw_block(self)
    }
    /// Try and convert a raw block into a pool header
    pub fn from_block(block: &RawBlock) -> Result<PoolDiskHeader, HeaderError> {
        pool_header_from_raw_block(block)
    }
}

/// This function bypasses the usual disk types.
/// I dont want to rewrite this right now. It'll do.
fn read_pool_header_from_disk() -> Result<PoolDiskHeader, DriveError> {
    // Get the header block from the pool disk (disk 0)
    // If the header is missing, and there is no fluster magic, ask if we are creating a new pool.

    // This function will return an error if reading in the pool is determined to be impossible due to
    // factors outside of our control.

    // Get block 0 of disk 0

    // We will contain all of our logic within a loop, so if the user inserts the incorrect disk we can ask for another, etc
    // This is messy. Sorry.

    loop {
        // if we are running with virtual disks, we skip the prompt.
        if !USE_VIRTUAL_DISKS
            .try_lock()
            .expect("Fluster is single threaded.")
            .is_some()
        {
            // Not using virtual disks, prompt the user...
            let result =
                rprompt::prompt_reply("Please insert the pool root disk (Disk 0), then press enter. Or type \"wipe\" to enter disk wiper mode: ").expect(
                    "prompting should not fail."
                );

            // This is the only chance the user gets to enter disk wiping mode.
            // Why are we doing this in pool/header_methods ? idk.

            if result.contains("wipe") {
                disk_wiper_mode()
            }
        }

        // User wants to open this disk for the pool.

        // Attempt to extract the header
        let some_disk = FloppyDrive::open_direct(0)?;

        // We've now read in either the PoolDisk, or some other type of disk.
        // Find out what it is.

        match some_disk {
            crate::pool::disk::drive_struct::DiskType::Pool(pool_disk) => {
                // This is what we want!
                return Ok(pool_disk.header);
            }
            crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => {
                // For any disk type other than Blank, we will ask if user wants to wipe it.
                display_info_and_ask_wipe(&mut DiskType::Standard(standard_disk))?;
                // Start the loop over, if they wiped the disk, the outcome will change.
                continue;
            }
            crate::pool::disk::drive_struct::DiskType::Unknown(file) => {
                display_info_and_ask_wipe(&mut DiskType::Unknown(file))?;
                continue;
            }
            crate::pool::disk::drive_struct::DiskType::Blank(disk) => {
                // The disk is blank, we will ask if the user wants to create a new pool.
                prompt_for_new_pool(disk)?;
                // The user either created a new pool, or didnt, so we just continue and run through this again.
                continue;
            }
        }
    }
}

/// Ask the user if they want to create a new pool with the currently inserted disk.
/// If so, we blank out the disk
fn prompt_for_new_pool(disk: BlankDisk) -> Result<(), DriveError> {
    // if we are running with virtual disks, we skip the prompt.
    debug!("Locking USE_VIRTUAL_DISKS...");
    if USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
        .is_some()
    {
        debug!("We are running with virtual disks, skipping the new pool prompt.");
        // Using virtual disks, we are going to create the pool immediately.
        return create_new_pool_disk(disk);
    } else {
        // If we are running a test, we should never be asking for user input, thus we should always
        // be using virtual disks.
        assert!(!cfg!(test));
    }

    // Ask the user if they want to create a new pool starting on this disk (hereafer disk 0 / root disk)
    println!("This disk is blank. Do you wish to create a new pool?");
    loop {
        let reply = rprompt::prompt_reply("y/n: ").expect("prompts should not fail");
        if reply.to_lowercase().starts_with('y') {
            break;
        } else if reply.to_lowercase().starts_with('n') {
            // They dont wanna make a new one.
            return Ok(());
        }
        println!("Try again.")
    }

    // User said yes. Make the disk.
    create_new_pool_disk(disk)
}

fn pool_header_from_raw_block(block: &RawBlock) -> Result<PoolDiskHeader, HeaderError> {
    // As usual, check for the magic
    if !check_for_magic(&block.data) {
        // There is no magic, thus this cannot be a header

        // The disk may be blank though.
        if block.data.iter().all(|byte| *byte == 0) {
            // Disk is blank
            return Err(HeaderError::Blank);
        }

        // Something else is wrong.
        return Err(HeaderError::Invalid);
    }

    // Easier alignment
    let mut offset: usize = 8;

    // Pool headers always have bit 7 set in the flags, other headers are forbidden from writing this bit.
    let flags: PoolHeaderFlags = match PoolHeaderFlags::from_bits(block.data[offset]) {
        Some(ok) => ok,
        None => {
            // extra bits in the flags were set, either this isn't a pool header, or it is corrupted in some way.
            return Err(HeaderError::Invalid);
        }
    };

    // Make sure the pool header bit is indeed set.
    if !flags.contains(PoolHeaderFlags::RequiredHeaderBit) {
        // The header must have missed the joke, since it didn't quite get the bit.
        // Not a pool header.
        return Err(HeaderError::Invalid);
    }

    offset += 1;

    // Now we can actually start extracting the header.

    // Highest disk
    let highest_known_disk: u16 =
        u16::from_le_bytes(block.data[offset..offset + 2].try_into().expect("Impossible"));

    offset += 2;

    // Disk with next free block
    let disk_with_next_free_block: u16 =
        u16::from_le_bytes(block.data[offset..offset + 2].try_into().expect("Impossible"));

    offset += 2;

    // Blocks free in pool
    let pool_standard_blocks_free: u32 =
        u32::from_le_bytes(block.data[offset..offset + 4].try_into().expect("Impossible"));

    // Block allocation map
    // Stop using the offset since this is always at the end.
    let block_usage_map: [u8; 360] = block.data[148..148 + 360].try_into().expect("Impossible");

    // The latest inode write is not persisted between launches, so we point at the root inode.
    let latest_inode_write: DiskPointer = DiskPointer { disk: 1, block: 1 };

    Ok(PoolDiskHeader {
        flags,
        highest_known_disk,
        disk_with_next_free_block,
        pool_standard_blocks_free,
        latest_inode_write, // This is not persisted between launches.
        block_usage_map,
    })
}

fn pool_header_to_raw_block(header: PoolDiskHeader) -> RawBlock {
    // Deconstruct / discombobulate
    #[deny(unused_variables)] // You need to write ALL of them.
    let PoolDiskHeader {
        flags,
        highest_known_disk,
        disk_with_next_free_block,
        pool_standard_blocks_free,
        latest_inode_write,
        block_usage_map,
    } = header;

    // Create buffer for the header
    let mut buffer: [u8; 512] = [0u8; 512];

    // The magic
    buffer[0..8].copy_from_slice("Fluster!".as_bytes());

    // offset for easier alignment
    let mut offset: usize = 8;

    // Flags
    buffer[offset] = flags.bits();
    offset += 1;

    // Highest known disk
    buffer[offset..offset + 2].copy_from_slice(&highest_known_disk.to_le_bytes());
    offset += 2;

    // Disk with next free block
    buffer[offset..offset + 2].copy_from_slice(&disk_with_next_free_block.to_le_bytes());
    offset += 2;

    // Free blocks
    buffer[offset..offset + 4].copy_from_slice(&pool_standard_blocks_free.to_le_bytes());

    // We do not save the inode write disk information.
    let _ = latest_inode_write;

    // Block usage map
    // Doesn't use offset, static location.
    buffer[148..148 + 360].copy_from_slice(&block_usage_map);

    // Add the CRC
    // TODO: Make sure there is a test for valid crcs on this header type
    add_crc_to_block(&mut buffer);

    // This needs to always go at block 0
    let block_origin: DiskPointer = DiskPointer {
        disk: 0, // Pool disk is always disk 0
        block: 0, // Header is always at block 0
    };

    RawBlock {
        block_origin,
        data: buffer,
    }
}

fn create_new_pool_disk(mut disk: BlankDisk) -> Result<(), DriveError> {
    // Time for a brand new pool!
    debug!("A new pool disk was created.");
    // We will create a brand new header, and write that header to the disk.
    let new_header = new_pool_header();

    // Now we need to write that
    let writeable_block: RawBlock = new_header.to_block();

    // Write it to the disk!
    // This is unchecked because the disk header does not exist yet, we cannot allocate space
    // for the header without the header.

    // We dont use cached IO here, since pool disks cannot have any caching on them.
    
    disk.unchecked_write_block(&writeable_block)?;

    // Done!
    Ok(())
}

// Brand new pool header
fn new_pool_header() -> PoolDiskHeader {
    // Default pool header

    // Flags
    let mut flags: PoolHeaderFlags = PoolHeaderFlags::empty();
    // Needs the required bit
    flags.insert(PoolHeaderFlags::RequiredHeaderBit);

    // The highest known disk for a brand new pool is the root disk itself, zero.
    let highest_known_disk: u16 = 0;

    // The disk with the next free block.
    // Starts at one to skip the pool disk.
    let disk_with_next_free_block: u16 = 1;

    // How many pool blocks are free? None! We only have the root disk!
    let pool_standard_blocks_free: u32 = 0;

    // What blocks are free on the pool disk? Not the first one!
    let mut block_usage_map: [u8; 360] = [0u8; 360];
    block_usage_map[0] = 0b10000000;

    // Everything is empty, so the latest write is just gonna be the root inode.
    let latest_inode_write: DiskPointer = DiskPointer { disk: 1, block: 1 };

    PoolDiskHeader {
        flags,
        highest_known_disk,
        disk_with_next_free_block,
        pool_standard_blocks_free,
        latest_inode_write, // This is not persisted on disk.
        block_usage_map,
    }
}

/// Put that pool away
fn write_pool_header_to_disk(header: &PoolDiskHeader) -> Result<(), DriveError> {
    // Make a block
    let header_block = header.to_block();

    // Get the pool disk
    #[allow(deprecated)] // Pool disks cannot use the cache.
    let mut disk: PoolDisk = match FloppyDrive::open(0)? {
        DiskType::Pool(pool_disk) => pool_disk,
        _ => unreachable!("Disk 0 should NEVER be assigned to a non-pool disk!"),
    };

    // Replace the header
    disk.header = *header;
    
    // Write it.
    // We cant use the usual disk.flush() since that usually uses cache methods. Pool disk blocks are not cached.
    disk.unchecked_write_block(&header_block)?;

    // All done.
    Ok(())
}

/// Disk wiper mode
fn disk_wiper_mode() -> ! {
    // Time to wipe some disks!
    println!("Welcome to disk wiper mode!");
    loop {
        let are_we_done_yet =
            rprompt::prompt_reply("Please insert the next disk you would like to wipe, then hit enter. Or type `exit`.").expect("STDIO moment, should not fail");
        if are_we_done_yet.contains("exit") {
            // User is bored.
            break;
        }
        // Wiping another disk.
        println!("Wiping the inserted disk, please wait...");
        
        // Get the disk from the drive. Disk numbers do not matter here.
        let mut disk = match FloppyDrive::open_direct(0) {
            Ok(ok) => ok,
            Err(err) => {
                // Uh oh

                // If there just isn't a disk in the drive, we can continue
                if err == DriveError::DriveEmpty {
                    println!("Cant wipe nothing bozo. Put a disk in!");
                    continue;
                }

                println!("Opening the disk failed. Here's why:");
                println!("{err:#?}");
                break; // cannot go further if the drive is angry.
            },
        };

        // Wipe the disk
        match destroy_disk(disk.disk_file_mut()) {
            Ok(_) => {},
            Err(err) => {
                // Uh oh
                println!("Wiping the disk failed. Here's why:");
                println!("{err:#?}");
                match err {
                    DriveError::DriveEmpty => {},
                    DriveError::Retry => {},
                    DriveError::TakingTooLong => {
                        println!("That disk is responding VERY slowly to writes, its probably bad.")
                    },
                }
                break;
            },
        }

        println!("Disk wiped.");
    }

    // Done wiping disks, user must restart Fluster.
    println!("Exiting disk wiping mode. You must restart fluster.");
    exit(0);
}