// Methods that are generic across all types of disk.

// Using the floppy drive interface should work like this:
// Request a disk, get back a DiskType that matches the number provided.


// Imports

use crate::pool::disk::generic::block::block_structs::BlockError;
use crate::pool::disk::generic::io::read::read_block_direct;

use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;

use crate::pool::disk::dense_disk::dense_disk_struct::DenseDisk;

use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;

use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::filesystem::filesystem_struct::FLOPPY_PATH;

use super::drive_struct::FloppyDriveError;
use super::drive_struct::FloppyDrive;
use super::drive_struct::DiskType;

use std::fs::OpenOptions;
use std::fs::File;

// Implementations


/// Various operations on the underlying Disk.
/// This is meant to be high level, just enough to get to the disk type below.
impl FloppyDrive {
    /// Open the disk currently in the drive, regardless of disk type.
    /// This should only be used when initializing the pool. Use open() instead.
    pub fn open_direct(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
        open_and_deduce_disk(disk_number)
    }
    
    /// Opens a specific disk, or waits until the user inserts that disk.
    pub fn open(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
        prompt_for_disk(disk_number)
    }
}




// Functions for implementations

fn open_and_deduce_disk(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
    // First, we need the file to read from
    let disk_file: File = get_floppy_drive_file(disk_number)?;

    // Now we must get the 0th block
    // We need to read a block before we have an actual disk, so we need
    // to call this function directly as a workaround.
    // We must ignore the CRC here, since we know nothing about the disk.
    let header_block = read_block_direct(disk_file, 0, false)?;

    // Now we check for the magic
    if !check_for_magic(header_block.data) {
        // The magic is missing, check if the block is empty
        if header_block.data.iter().all(|byte| *byte == 0) {
            // Block is completely blank.
            return Ok(DiskType::Blank)
        }
        // Otherwise, we dont know what kind of disk this is.
        // Its probably not a fluster disk.
        return Ok(DiskType::Unknown)
    }

    // Magic exists, time to figure out what kind of disk this is.
    // Bitflags will tell us.


    // Pool disk.
    if header_block[8] & 0b10000000 != 0 {
        return Ok(DiskType::Pool(PoolDisk::bootstrap(header_block)))
    }

    // Dense disk.
    if header_block[8] & 0b01000000 != 0 {
        return Ok(DiskType::Dense(DenseDisk::bootstrap(header_block)))
    }

    // Standard disk.
    if header_block[8] & 0b00100000 != 0 {
        return Ok(DiskType::Standard(StandardDisk::bootstrap(header_block)))
    }
    
    // it should be impossible to get here
    unreachable!();
}




/// Get the path of the floppy drive
fn get_floppy_drive_file(disk_number: u16) -> Result<File, FloppyDriveError> {
    // If someone wants to port this to another operating system, this function will need appropriate changes
    // to remove its dependency on getting the raw floppy device from Windows.

    // TODO: Prevent blocking (Return NoDiskInserted if file does not load in under 1 second.)

    // If we are running with virtual disks enabled, we are going to use a temp folder instead of the actual disk to speed up
    // development, waiting for disk seeks is slow and loud lol.

    if let Some(ref path) = *USE_VIRTUAL_DISKS.lock().expect("Fluster is single threaded.") {
        println!("Attempting to access virtual disk {disk_number}...");
        // Get the tempfile.
        // These files do not delete themselves.

        // if disk 0 is missing, we need to make it,
        // because the pool cannot create disk 0 without first loading itself... from disk 0.
        let _ = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join("disk0.fsr"))?;

        // If the tempfile does not exist, that means `create` was never called, which is an issue.
        // This should never be allowed, so an unwrap is okay in this case.

        let temp_disk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false) // We will panic if the disk does not exist.
            .truncate(false)
            .open(path.join(format!("disk{disk_number}.fsr"))).expect("Disks should be created before read.");


        // Make sure the file is one floppy big, should have no effect on pre-existing files, since
        // they will already be this size.
        temp_disk_file.set_len(512 * 2880)?; 

        return Ok(temp_disk_file);
    }

    // Get the global path to the floppy disk drive
    let disk_path = FLOPPY_PATH.lock().expect("Fluster is single threaded.").clone();

    // Open the disk, or return an error from it
    match OpenOptions::new().read(true).write(true).open(disk_path) {
        Ok(ok) => Ok(ok),
        // Convert that into a BlockError, since this is an IO operation... Kinda?
        Err(error) => Err(BlockError::from(error).into()),
    }
}

/// Look for the magic "Fluster!" string.
pub fn check_for_magic(block_bytes: &[u8]) -> bool {
    // is the "Fluster!" magic present?
    block_bytes[0..8] == *"Fluster!".as_bytes()
}


/// Prompt user to insert the disk we want.
/// If the disk is already in the drive, no prompt will happen.
/// Will error out for non-wrong disk related issues.
/// This function does not disable the CRC check, you must use open() if you are ignoring CRC.
fn prompt_for_disk(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
    let mut is_user_an_idiot: bool = false; // Did the user put in the wrong disk when asked?
    let mut disk: Result<DiskType, FloppyDriveError>;
    loop {
        // Try opening the current disk
        disk = open_and_deduce_disk(disk_number);
        // Is this the correct disk?

        if let Ok(ok) = disk {
            // Check if this is the right disk number
            if disk_number == ok.get_disk_number() {
                // Thats the right disk!
                return ok;
            }
        }
        
        // This was not the right disk.
        // Prompt user to swap disks.

        if is_user_an_idiot {
            println!("Wrong disk. Try again.");
        } else {
            is_user_an_idiot = true;
        }
        let _ = rprompt::prompt_reply(format!("Please insert disk {disk_number}, then press enter."));
    }
}