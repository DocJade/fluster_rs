// Imports
use std::{
    fs::{File, OpenOptions}, io::Read, u16
};

use log::error;

use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;

// Implementations

// !! Only numbered options should be public! !!

impl StandardDisk {
    //
    //  Private functions
    //

    // Open the disk currently connected to the system
    // Ensures the disk opened matches the provided ID.
    fn open_numbered(disk_number: u16) -> Result<Self, DiskError> {
        // Opening numbered disks does not ignore the header, as we need to check it for the disk number.
        open_numbered(disk_number, false)
    }

    // Used to initlize the data on a disk, ==not public==.
    fn initialize_numbered(disk: &mut Self, disk_number: u16) -> Result<(), DiskError> {
        initialize_numbered(disk, disk_number)
    }

    //
    //  Public functions
    //

    /// Opens the current disk in the drive directly.
    /// The returned disk will have a spoofed header of all <T>::MAX or other equivalents.
    /// Does not check disk number, we assume the correct disk is in the drive. It is the callers responsibility to check.
    /// Does not check headers.
    /// Does not check CRC.
    pub fn unchecked_open(disk_number: u16) -> Result<Disk, DiskError> {
        // We must ignore the header, otherwise we would be checking for a header, which can fail, or CRC could fail.
        open_numbered(disk_number, true)
    }

    /// Waits for user to insert the specified
    /// Will not return until specified disk is inserted, or if there are errors with the inserted disk.
    pub fn prompt_for_disk(disk_number: u16) -> Result<Disk, DiskError> {
        prompt_for_disk(disk_number)
    }

    /// Create a new disk. Destroys all data.
    /// Returns the new disk.
    /// Will not wipe disks that contain a header.
    pub fn create(disk_number: u16) -> Result<Disk, DiskError> {
        create(disk_number)
    }

    /// Destroys ALL data on a disk.
    /// Obviously, this cannot be undone.
    pub fn wipe(&self) -> Result<(), DiskError> {
        wipe(self)
    }
}


/// Ocasionally, we need to create fake headers during disk loading.
impl StandardDiskHeader {
    fn spoof() -> Self {
        Self {
            flags: HeaderFlags::from_bits_retain(0b11111111),
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
fn create(disk_number: u16) -> Result<Disk, DiskError> {
    // Get the current disk
    let new_disk_file = get_disk_file(0)?;

    // Spoof the header, since we're about to give it a new one.
    let mut disk: Disk = Disk {
        number: disk_number,
        header: DiskHeader::spoof(),
        disk_file: new_disk_file,
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
fn initialize_numbered(disk: &mut Disk, disk_number: u16) -> Result<(), DiskError> {
    // A new, fresh disk!

    // Read in the first block of the disk and ensue its empty.
    // CRC is disabled, since we only care if the block is blank.
    let block = disk.read_block(0, true)?;

    // Check if blank
    if !block.data.iter().all(|byte| *byte == 0) {
        // Disk was not blank.
        return Err(DiskError::NotBlank);
    }

    // Wipe the entire disk
    disk.wipe()?;

    // Time to write in all of the header data.
    // Construct the new header

    // New disks have no flags set.
    let flags: StandardHeaderFlags = StandardHeaderFlags::empty();

    // New disks do have a few pre-allocated blocks, namely the header and the first inode block
    // So construct a map accordingly
    let mut block_usage_map: [u8; 360] = [0u8; 360];
    block_usage_map[0] = 0b11000000; // TODO: Document that the block map is indexed literally, as in block 0 is the first bit.

    let header = DiskHeader {
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

//
// Error type conversion
//

impl From<std::io::Error> for DiskError {
    fn from(value: std::io::Error) -> Self {
        // Just cast it to a block error lol
        BlockError::from(value).into()
    }
}