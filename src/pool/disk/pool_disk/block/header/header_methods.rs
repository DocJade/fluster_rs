// So no head?

// Imports

use log::debug;
use log::warn;

use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::drive_methods::check_for_magic;
use crate::pool::disk::drive_methods::display_info_and_ask_wipe;
use crate::pool::disk::drive_struct::DiskType;
use crate::pool::disk::drive_struct::FloppyDrive;
use crate::pool::disk::drive_struct::FloppyDriveError;
use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::generic::block::crc::add_crc_to_block;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolHeaderFlags;

use super::header_struct::PoolDiskHeader;
use super::header_struct::PoolHeaderError;

// Implementations

impl PoolDiskHeader {
    /// Reterive the header from the pool disk
    pub fn read() -> Result<Self, FloppyDriveError> {
        read_pool_header_from_disk()
    }
    /// Convert the pool header into a RawBlock
    pub fn to_block(self) -> RawBlock {
        pool_header_to_raw_block(self)
    }
    /// Try and convert a raw block into a pool header
    pub fn from_block(block: &RawBlock) -> Result<PoolDiskHeader, PoolHeaderError> {
        pool_header_from_raw_block(block)
    }
}

/// This function bypasses the usual disk types.
/// I dont want to rewrite this right now. It'll do.
fn read_pool_header_from_disk() -> Result<PoolDiskHeader, FloppyDriveError> {
    // Get the header block from the pool disk (disk 0)
    // If the header is missing, and there is no fluster magic, ask if we are creating a new pool.

    // This function will return an error if reading in the pool is determined to be impossible due to
    // factors outside of our control.

    // Get block 0 of disk 0

    // if we are running with virtual disks, we skip the prompt.
    if !USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
        .is_some()
    {
        // Not using virtual disks, prompt the user...
        let _ =
            rprompt::prompt_reply("Please insert the pool root disk (Disk 0), then press enter.");
    }

    // We will contain all of our logic within a loop, so if the user inserts the incorrect disk we can ask for another, etc
    // This is messy. Sorry.

    loop {
        // Attempt to extract the header
        // First we need to open the disk, which can fail for various reasons.
        // This is the unchecked method, so we will need to read the header off ourself.
        // If this fails, check if its unrecoverable, if it isnt, handle that
        let some_disk = match FloppyDrive::open_direct(0) {
            Ok(ok) => ok,
            Err(error) => {
                // Opening the disk failed, check if we can recover
                check_for_external_error(&error)?;
                // We are still here, error was not fatal, try again...
                warn!("Reading opening the disk failed, but was not fatal.");
                warn!("Error type: {error}");
                // Try again
                continue;
            }
        };

        // We've now read in either the PoolDisk, or some other type of disk.
        // Find out what it is.

        match some_disk {
            crate::pool::disk::drive_struct::DiskType::Pool(pool_disk) => {
                // This is what we want!
                return Ok(pool_disk.header);
            }
            crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => {
                // For any disk type other than Blank, we will ask if user wants to wipe it.
                display_info_and_ask_wipe(DiskType::Standard(standard_disk))?;
                // Start the loop over, if they wiped the disk, the outcome will change.
                continue;
            }
            crate::pool::disk::drive_struct::DiskType::Dense(dense_disk) => {
                display_info_and_ask_wipe(DiskType::Dense(dense_disk))?;
                continue;
            }
            crate::pool::disk::drive_struct::DiskType::Unknown(file) => {
                display_info_and_ask_wipe(DiskType::Unknown(file))?;
                continue;
            }
            crate::pool::disk::drive_struct::DiskType::Blank(disk) => {
                // The disk is blank, we will ask if the user wants to create a new pool.
                prompt_for_new_pool(disk)?;
                // The user either created a new pool, or didnt, so we just continue and run through this again.
                continue;
            }
        }

        // One of the branch arms has to be hit.
        unreachable!();
    }
}

/// Ask the user if they want to create a new pool with the currently inserted disk.
/// If so, we blank out the disk
fn prompt_for_new_pool(disk: BlankDisk) -> Result<(), FloppyDriveError> {
    // if we are running with virtual disks, we skip the prompt.
    debug!("Locking USE_VIRTUAL_DISKS...");
    if USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
        .is_some()
    {
        debug!("We are running with virtual disks, skipping the new pool prompt.");
        // Using virtual disks, we are going to create the pool immediately.
        create_new_pool_disk(disk)?;
        // Return, next loop will catch that this has changed.
        return Ok(());
    } else {
        // If we are running a test, we should never be asking for user input, thus we should always
        // be using virtual disks.
        assert!(!cfg!(test));
    }

    // Ask the user if they want to create a new pool starting on this disk (hereafer disk 0 / root disk)
    println!("This disk is blank. Do you wish to create a new pool?");
    loop {
        let reply = rprompt::prompt_reply("y/n: ")?; // Weirdly enough, this is an IO error, lol
        if reply.to_lowercase().starts_with('y') {
            break;
        } else if reply.to_lowercase().starts_with('n') {
            // They dont wanna make a new one.
            return Ok(());
        }
        println!("Try again.")
    }

    todo!()
}

fn pool_header_from_raw_block(block: &RawBlock) -> Result<PoolDiskHeader, PoolHeaderError> {
    // As usual, check for the magic
    if !check_for_magic(&block.data) {
        // There is no magic, thus this cannot be a header

        // The disk may be blank though.
        if block.data.iter().all(|byte| *byte == 0) {
            // Disk is blank
            return Err(PoolHeaderError::Blank);
        }

        // Something else is wrong.
        return Err(PoolHeaderError::Invalid);
    }

    // Pool headers always have bit 7 set in the flags, other headers are forbidden from writing this bit.
    let flags: PoolHeaderFlags = match PoolHeaderFlags::from_bits(block.data[8]) {
        Some(ok) => ok,
        None => {
            // extra bits in the flags were set, either this isn't a pool header, or it is corrupted in some way.
            return Err(PoolHeaderError::Invalid);
        }
    };

    // Make sure the pool header bit is indeed set.
    if !flags.contains(PoolHeaderFlags::RequiredHeaderBit) {
        // The header must have missed the joke, since it didn't quite get the bit.
        // Not a pool header.
        return Err(PoolHeaderError::Invalid);
    }

    // Now we can actually start extracting the header.

    // Highest disk
    let highest_known_disk: u16 =
        u16::from_le_bytes(block.data[9..9 + 2].try_into().expect("Impossible"));

    // Disk with next free block
    let disk_with_next_free_block: u16 =
        u16::from_le_bytes(block.data[11..11 + 2].try_into().expect("Impossible"));

    // Blocks free in pool
    let pool_standard_blocks_free: u16 =
        u16::from_le_bytes(block.data[13..13 + 2].try_into().expect("Impossible"));

    // Block allocation map
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

    // Flags
    buffer[8] = flags.bits();

    // Highest known disk
    buffer[9..9 + 2].copy_from_slice(&highest_known_disk.to_le_bytes());

    // Disk with next free block
    buffer[11..11 + 2].copy_from_slice(&disk_with_next_free_block.to_le_bytes());

    // Free blocks
    buffer[13..13 + 2].copy_from_slice(&pool_standard_blocks_free.to_le_bytes());

    // We do not save the inode write disk information.
    let _ = latest_inode_write;

    // Block usage map
    buffer[148..148 + 360].copy_from_slice(&block_usage_map);

    // Add the CRC
    // TODO: Make sure there is a test for valid crcs on this header type
    add_crc_to_block(&mut buffer);

    // This needs to always go at block 0
    RawBlock {
        block_index: 0,
        data: buffer,
        originating_disk: None, // This is on its way to be written.
    }
}

fn create_new_pool_disk(mut disk: BlankDisk) -> Result<(), FloppyDriveError> {
    // Time for a brand new pool!
    debug!("A new pool disk was created.");
    // We will create a brand new header, and write that header to the disk.
    let new_header = new_pool_header();

    // Now we need to write that
    let writeable_block: RawBlock = new_header.to_block();

    // Write it to the disk!
    // This is unchecked because the disk header does not exist yet, we cannot allocate space
    // for the header without the header.
    disk.unchecked_write_block(&writeable_block)?;

    // Done!
    Ok(())
}

/// Returns () if we can recover, use ? to percolate the error easily.
fn check_for_external_error(error: &FloppyDriveError) -> Result<(), FloppyDriveError> {
    // There are certian types of errors we cannot recover from when reading in disks.
    // Reasons are documented next to their corresponding match arms.

    warn!(
        "Encountered an error while attempting to get information about the currently inserted floppy."
    );
    warn!("{error}");

    // TODO: I just wanna get things working, i'll fix this later.
    match error {
        FloppyDriveError::Uninitialized => todo!(),
        FloppyDriveError::NotBlank => todo!(),
        FloppyDriveError::WipeFailure => todo!(),
        FloppyDriveError::WrongDisk => todo!(),
        FloppyDriveError::BadHeader(header_conversion_error) => {
            todo!("{header_conversion_error:#?}")
        }
        FloppyDriveError::BlockError(block_error) => todo!("{block_error:#?}"),
    }
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

    // The disk with the next free block set to the pool disk itself.
    // The pool block allocator will skip any disks that are not Standard,
    // So it will just skip over this the first time we use it.
    let disk_with_next_free_block: u16 = 0;

    // How many pool blocks are free? None! We only have the root disk!
    let pool_standard_blocks_free: u16 = 0;

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
