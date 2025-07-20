// Methods that are generic across all types of disk.

// Using the floppy drive interface should work like this:
// Request a disk, get back a DiskType that matches the number provided.

// Imports

use log::debug;
use log::error;
use log::warn;

use crate::helpers::hex_view::hex_view;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::drive_struct::DiskBootstrap;
use crate::pool::disk::generic::block::block_structs::BlockError;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::io::read::read_block_direct;

use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;

use crate::pool::disk::dense_disk::dense_disk_struct::DenseDisk;

use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;

use crate::filesystem::filesystem_struct::FLOPPY_PATH;
use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::disk::unknown_disk::unknown_disk_struct::UnknownDisk;

use super::drive_struct::DiskType;
use super::drive_struct::FloppyDrive;
use super::drive_struct::FloppyDriveError;

use std::fs::File;
use std::fs::OpenOptions;

// Implementations

/// Various operations on the underlying Disk.
/// This is meant to be high level, just enough to get to the disk type below.
impl FloppyDrive {
    /// Open the disk currently in the drive, regardless of disk type.
    /// This should only be used when initializing the pool. Use open() instead.
    pub fn open_direct(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
        // This function does not create disks.
        open_and_deduce_disk(disk_number, false)
    }

    /// Opens a specific disk, or waits until the user inserts that disk.
    pub fn open(disk_number: u16) -> Result<DiskType, FloppyDriveError> {
        prompt_for_disk(disk_number)
    }

    /// Prompts the user for a blank floppy disk.
    pub fn get_blank_disk(disk_number: u16) -> Result<BlankDisk, FloppyDriveError> {
        prompt_for_blank_disk(disk_number)
    }
}

// Functions for implementations

fn open_and_deduce_disk(disk_number: u16, new_disk: bool) -> Result<DiskType, FloppyDriveError> {
    debug!("Opening and deducing disk disk {disk_number}...");
    debug!("Is it a new disk? : {new_disk}");
    // First, we need the file to read from
    let disk_file: File = get_floppy_drive_file(disk_number, new_disk)?;
    
    // Now we must get the 0th block
    // We need to read a block before we have an actual disk, so we need
    // to call this function directly as a workaround.
    // We must ignore the CRC here, since we know nothing about the disk.
    debug!("Reading in the header at block 0...");
    let header_block = read_block_direct(&disk_file, 0, true)?;
    
    // Now we check for the magic
    debug!("Checking for magic...");
    if !check_for_magic(&header_block.data) {
        debug!("No magic, checking if its blank...");
        // The magic is missing, check if the block is empty
        if header_block.data.iter().all(|byte| *byte == 0) {
            // Block is completely blank.
            debug!("Disk is blank, returning.");
            return Ok(DiskType::Blank(BlankDisk::new(disk_file)));
        }
        // Otherwise, we dont know what kind of disk this is.
        // Its probably not a fluster disk.
        debug!("Disk was not blank, returning unknown disk...");
        return Ok(DiskType::Unknown(UnknownDisk::new(disk_file)));
    }
    
    // Magic exists, time to figure out what kind of disk this is.
    debug!("Disk has magic, deducing type...");
    // Bitflags will tell us.
    
    // Pool disk.
    // The header reads should check the CRC of the block.
    if header_block.data[8] & 0b10000000 != 0 {
        debug!("Head is for a pool disk, returning.");
        return Ok(DiskType::Pool(PoolDisk::from_header(
            header_block,
            disk_file,
        )));
    }
    
    // Dense disk.
    if header_block.data[8] & 0b01000000 != 0 {
        debug!("Head is for a dense disk, returning.");
        return Ok(DiskType::Dense(DenseDisk::from_header(
            header_block,
            disk_file,
        )));
    }
    
    // Standard disk.
    if header_block.data[8] & 0b00100000 != 0 {
        debug!("Head is for a standard disk, returning.");
        return Ok(DiskType::Standard(StandardDisk::from_header(
            header_block,
            disk_file,
        )));
    }
    
    // it should be impossible to get here
    error!("Header of disk did not match any known disk type!");
    error!("Hexdump:\n{}",hex_view(header_block.data.to_vec()));
    error!("We cannot continue with an un-deducible disk!");
    unreachable!();
}

/// Get the path of the floppy drive
fn get_floppy_drive_file(disk_number: u16, new_disk: bool) -> Result<File, FloppyDriveError> {
    // If someone wants to port this to another operating system, this function will need appropriate changes
    // to remove its dependency on getting the raw floppy device from Windows.

    // TODO: Prevent blocking (Return NoDiskInserted if file does not load in under 1 second.)

    // If we are running with virtual disks enabled, we are going to use a temp folder instead of the actual disk to speed up
    // development, waiting for disk seeks is slow and loud lol.

    debug!("Locking USE_VIRTUAL_DISKS...");
    if let Some(ref path) = *USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
    {
        debug!("Attempting to access virtual disk {disk_number}...");
        debug!("Are we creating this disk? : {new_disk}");
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
        // This will create the disk if the correct argument is passed.

        debug!("Opening the temp disk with read/write privileges...");
        let temp_disk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(new_disk) // We will panic if the disk does not exist, unless told to create it.
            .truncate(false)
            .open(path.join(format!("disk{disk_number}.fsr")))
            .expect("Disks should be created before read.");

        // Make sure the file is one floppy big, should have no effect on pre-existing files, since
        // they will already be this size.
        debug!("Attempting to resize the temporary file to floppy size...");
        temp_disk_file.set_len(512 * 2880)?;
        
        debug!("Returning virtual disk.");
        return Ok(temp_disk_file);
    }

    // Get the global path to the floppy disk drive
    debug!("Locking FLOPPY_PATH...");
    let disk_path = FLOPPY_PATH
        .try_lock()
        .expect("Fluster is single threaded.")
        .clone();

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
    debug!("Prompting for disk {disk_number}...");
    let mut is_user_an_idiot: bool = false; // Did the user put in the wrong disk when asked?
    let mut disk: Result<DiskType, FloppyDriveError>;
    loop {
        // Try opening the current disk.
        // We do not create disks here.
        disk = open_and_deduce_disk(disk_number, false);
        // Is this the correct disk?
        
        match disk {
            Ok(ok) => {
                // Check if this is the right disk number
                if disk_number == ok.get_disk_number() {
                    // Thats the right disk!
                    debug!("Got the correct disk.");
                    return Ok(ok);
                }
                warn!("Wrong disk received. Got disk {}", ok.get_disk_number());
            },
            Err(error) => match error {
                // If the error isn't about it being the wrong disk, we need to throw the error up.
                FloppyDriveError::WrongDisk => {},
                _ => {
                    warn!("Got an error while prompting for disk: {error}");
                    return Err(error);
                }
            },
        }

        // This was not the right disk.
        // We should ALWAYS get the correct disk when testing.
        #[cfg(test)]
        if cfg!(test) {
            error!("Got an invalid disk during a test!");
            panic!("Test received an invalid disk!");
        }
        
        // Prompt user to swap disks.

        if is_user_an_idiot {
            println!("Wrong disk. Try again.");
        } else {
            is_user_an_idiot = true;
        }
        let _ = rprompt::prompt_reply(format!(
            "Please insert disk {disk_number}, then press enter."
        ));
    }
}

// get a blank disk
fn prompt_for_blank_disk(disk_number: u16) -> Result<BlankDisk, FloppyDriveError> {
    // Pester user for a blank disk
    let mut try_again: bool = false;

    // If we are on virtual disks, skip the initial prompt
    if !USE_VIRTUAL_DISKS.try_lock().expect("Fluster is single threaded.").is_some() {
        let _ = rprompt::prompt_reply("That disk is not blank. Please insert a blank disk, then hit enter.")?;
    }

    loop {
        if try_again {
            let _ = rprompt::prompt_reply("That disk is not blank. Please insert a blank disk, then hit enter.")?;
        }
        // we are making a new disk, so we must specify as such.
        let disk = open_and_deduce_disk(disk_number, true)?;
        match disk {
            // if its blank, all done
            DiskType::Blank(blank_disk) => return Ok(blank_disk),
            DiskType::Unknown(unknown_disk) => {
                // But if its an unknown disk, we can ask if the user would like to wipe their ass.
                display_info_and_ask_wipe(unknown_disk.into())?;
                // try again
                continue;
            },
            _ => {
                // This is not a blank disk.
                try_again = true;
            },
        }
    }
}

/// Takes in a non-blank disk and displays info about it, then asks the user if they would like to wipe the disk.
/// Wipes the disk if the user asks, returns nothing.
/// Will also return nothing if the user does not wipe the disk.
pub fn display_info_and_ask_wipe(disk: DiskType) -> Result<(), FloppyDriveError> {
    // This isn't a very friendly interface, but it'll do for now.

    // Display the disk type
    println!("The disk inserted is not blank. It is of type {disk:?}.");
    println!("Would you like to wipe this disk?");
    loop {
        let answer = rprompt::prompt_reply("y/n: ")?
            .to_ascii_lowercase()
            .contains('y');
        if answer {
            // Wipe time!
            todo!()
        } else {
            // No wipe.
            print!("Okay, this disk will not be wiped.");
            let _ = rprompt::prompt_reply("Please insert a different disk, then hit return.")?;
            return Ok(());
        }
    }
}


// Error conversion
impl From<std::io::Error> for FloppyDriveError {
    fn from(value: std::io::Error) -> Self {
        FloppyDriveError::BlockError(BlockError::from(value))
    }
}
