// So no head?

// Imports
use log::{info, warn};

use super::pool_header_struct::PoolHeader;
use super::pool_header_struct::PoolHeaderError;

// Implementations

impl PoolHeader {
    /// Reterive the header from the pool disk
    pub fn read() -> Result<Self, PoolHeaderError> {
        read_pool_header_from_disk()
    }
    /// Convert the pool header into a RawBlock
    pub fn to_block(&self) -> RawBlock {
        pool_header_to_raw_block(self)
    }
    /// Try and convert a raw block into a pool header
    pub fn from_block(block: &RawBlock) -> Result<PoolHeader, PoolHeaderError> {
        pool_header_from_raw_block(block)
    }
}

fn read_pool_header_from_disk() -> Result<PoolHeader, PoolHeaderError> {
    // Get the header block from the pool disk (disk 0)
    // If the header is missing, and there is no fluster magic, ask if we are creating a new pool.

    // This function will return an error if reading in the pool is determined to be impossible due to
    // factors outside of our control.

    // Get block 0 of disk 0

    // if we are running with virtual disks, we skip the prompt.
    if !USE_VIRTUAL_DISKS.lock().expect("Fluster is single threaded.").is_some() {
        // Not using virtual disks, prompt the user...
        let _ = rprompt::prompt_reply("Please insert the pool root disk (Disk 0), then press enter.");
    }

    // We will contain all of our logic within a loop, so if the user inserts the incorrect disk we can ask for another, etc
    // This is messy. Sorry.
    
    loop {
        // Attempt to extract the header
        // First we need to open the disk, which can fail for various reasons.
        // This is the unchecked method, so we will need to read the header off ourself.
        // If this fails, check if its unrecoverable, if it isnt, handle that
        let read_disk = match Disk::unchecked_open(0) {
            Ok(ok) => ok,
            Err(error) => {
                // Opening the disk failed, check if we can recover
                let _ = check_for_external_error(&error)?;
                // We are still here, error was not fatal, try again...
                info!("Reading opening the disk failed, but was not fatal.");
                info!("Error type: {error}");
                // Try again
                continue;
            },
        };

        // Read in block 0, which should contain the header.
        // We skip CRC checks, since we will check the CRC when reconstructing the header.
        let block: RawBlock = match read_disk.read_block(0, true) {
            Ok(ok) => ok,
            Err(error) => {
                // Reading the block failed...
                let _ = check_for_external_error(&error.clone().into())?;
                info!("Reading block 0 from the pool disk failed, but was not fatal.");
                info!("Error type: {error}");
                // Try again
                continue;
            },
        };
        
        // Get the header from that block
        // I'm in nesting hell
        let header: PoolHeader = match PoolHeader::from_block(&block) {
            Ok(ok) => {
                // That's the header we want! All done.
                return Ok(ok)
            },
            Err(error) => {
                // Extracting the header failed.
                match error {
                    PoolHeaderError::Invalid => {
                        // Ask the user if they want to wipe this disk
                        todo!()
                    },
                    PoolHeaderError::Blank => {
                        // Ask the user if they want to initialize the disk
                        match prompt_for_new_pool(read_disk) {
                            Ok(ok) => {
                                // Did they say yes?
                                if let Some(header) = ok {
                                    // Header was created and pool was initalized.
                                    return Ok(header);
                                };
                                // They said no :(
                                continue;
                            },
                            Err(error) => {
                                // The user either declined to create the pool, or an error occurred while creating it.
                                warn!("A new pool was attempted to be created, but the creation failed!");
                                warn!("Reason: {error}");
                                check_for_external_error(&error)?;
                                continue;
                            },
                        };
                    }
                }
            }
        };
    }
}


/// Ask the user if they want to create a new pool with the currently inserted disk.
/// If so, we blank out the disk
fn prompt_for_new_pool(disk: Disk) -> Result<Option<PoolHeader>, DiskError> {

    // if we are running with virtual disks, we skip the prompt.
    if USE_VIRTUAL_DISKS.lock().expect("Fluster is single threaded.").is_some() {
        // Using virtual disks, we are going to create the pool immediately.
        return Ok(Some(create_new_pool_disk(disk)?));
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
            break
        } else if reply.to_lowercase().starts_with('n') {
            // They dont wanna make a new one.
            return Ok(None)
        }
        println!("Try again.")
    }

    todo!()
}

fn pool_header_from_raw_block(block: &RawBlock) -> Result<PoolHeader, PoolHeaderError> {
    // As usual, check for the magic
    if !check_for_magic(&block.data) {
        // There is no magic, thus this cannot be a header

        // The disk may be blank though.
        if block.data.iter().all(|byte| *byte == 0) {
            // Disk is blank
            return Err(PoolHeaderError::Blank)
        }

        // Something else is wrong.
        return Err(PoolHeaderError::Invalid)
    }

    // Pool headers always have bit 7 set in the flags, other headers are forbidden from writing this bit.
    let flags: PoolHeaderFlags = match PoolHeaderFlags::from_bits(block.data[8]) {
        Some(ok) => ok,
        None => {
            // extra bits in the flags were set, either this isn't a pool header, or it is corrupted in some way.
            return Err(PoolHeaderError::Invalid)
        },
    };
    
    // Make sure the pool header bit is indeed set.
    if !flags.contains(PoolHeaderFlags::RequiredHeaderBit) {
        // The header must have missed the joke, since it didn't quite get the bit.
        // Not a pool header.
        return Err(PoolHeaderError::Invalid)
    }

    // Now we can actually start extracting the header.

    // Highest disk
    let highest_known_disk: u16 = u16::from_le_bytes(
            block.data[9..9 + 2]
            .try_into()
            .expect("Impossible")
    );

    // Disk with next free block
    let disk_with_next_free_block: u16 = u16::from_le_bytes(
            block.data[11..11 + 2]
            .try_into()
            .expect("Impossible")
    );

    
    // Blocks free in pool
    let pool_blocks_free: u16 = u16::from_le_bytes(
            block.data[13..13 + 2]
            .try_into()
            .expect("Impossible")
    );

    Ok(
        PoolHeader {
            flags,
            highest_known_disk,
            disk_with_next_free_block,
            pool_blocks_free,
        }
    )
}

fn pool_header_to_raw_block(header: &PoolHeader) -> RawBlock {

    // Deconstruct / discombobulate
    #[deny(unused_variables)] // You need to write ALL of them.
    let PoolHeader {
        flags,
        highest_known_disk,
        disk_with_next_free_block,
        pool_blocks_free,
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
    buffer[13..13 + 2].copy_from_slice(&pool_blocks_free.to_le_bytes());


    // Add the CRC
    // TODO: Make sure there is a test for valid crcs on this header type
    add_crc_to_block(&mut buffer);
    
    // This needs to always go at block 0
    RawBlock {
        block_index: 0,
        data: buffer,
    }
}

fn create_new_pool_disk(disk: Disk) -> Result<PoolHeader, DiskError> {
    // Time for a brand new pool!
    // We will create a brand new header, and write that header to the disk.
    let new_header = new_pool_header();

    // Now we need to write that
    let writeable_block: RawBlock = new_header.to_block();

    // Write it to the disk!
    disk.write_block(&writeable_block)?;
    
    // Done!
    Ok(new_header)
}

/// Returns () if we can recover, use ? to percolate the error easily.
fn check_for_external_error(error: &DiskError) -> Result<(), PoolHeaderError> {
    // There are certian types of errors we cannot recover from when reading in disks.
    // Reasons are documented next to their corresponding match arms.

    match error {
        DiskError::Uninitialized => todo!(),
        // This only happens if we are attempting to initalize a new disk to add to the pool, wont happen in here.
        DiskError::NotBlank => unreachable!(),
        // We do not call functions that check disk numbers, this variant is impossible.
        DiskError::WrongDisk => unreachable!(),
        // This result type is for the other kind of disk header.
        DiskError::BadHeader(_) => unreachable!(),
        DiskError::BlockError(block_error) => todo!(),
        DiskError::WipeFailure => todo!(),
    }

}

// Brand new pool header
fn new_pool_header() -> PoolHeader {
    // Default pool header

    // Flags
    let mut flags: PoolHeaderFlags = PoolHeaderFlags::empty();
    // Needs the required bit
    flags.insert(PoolHeaderFlags::RequiredHeaderBit);

    // The highest known disk for a brand new pool is the root disk itself, zero.
    let highest_known_disk: u16 = 0;

    // The disk with the next free block is, no disk!
    let disk_with_next_free_block: u16 = u16::MAX;

    // How many pool blocks are free? None! We only have the root disk!
    let pool_blocks_free: u16 = 0;

    PoolHeader {
        flags,
        highest_known_disk,
        disk_with_next_free_block,
        pool_blocks_free,
    }
}