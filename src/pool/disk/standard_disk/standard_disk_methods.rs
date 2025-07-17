// Imports
use std::{
    fs::{File, OpenOptions}, io::{Read, Seek}, os::unix::fs::FileExt, path::Path, process::exit, u16
};

use log::error;

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


// Just for this file, we ocasionally need to create a fake header.
impl DiskHeader {
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



/// Prompt user to insert the disk we want.
/// If the disk is already in the drive, no prompt will happen.
/// Will error out for non-wrong disk related issues.
/// This function does not disable the CRC check, you must use open() if you are ignoring CRC.
fn prompt_for_disk(disk_number: u16) -> Result<Disk, DiskError> {
    let mut is_user_an_idiot: bool = false; // Did the user put in the wrong disk when asked?
    let mut disk: Result<Disk, DiskError>;
    loop {
        // Try opening the current disk
        disk = open_numbered(disk_number, false);
        // Is this the correct disk?
        if disk.is_ok() {
            // yes it is
            return disk;
        } else if *disk.as_ref().err().expect("Checked.") != DiskError::WrongDisk { // Did we get an error we can't handle here?
            // Yep, percolate!
            return disk;
        }

        // This is the wrong disk, prompt user to swap disks.

        if is_user_an_idiot {
            println!("Wrong disk dumbass. Try again.");
        } else {
            is_user_an_idiot = true;
        }
        let _ = rprompt::prompt_reply(format!("Please insert disk {disk_number}, then press enter."));
    }
}

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

/// Get the path of the disk
fn get_disk_file(disk_number: u16) -> Result<File, DiskError> {
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
pub(crate) fn check_for_magic(block_bytes: &[u8]) -> bool {
    // is the "Fluster!" magic present?
    block_bytes[0..8] == *"Fluster!".as_bytes()
}

/// Returns header information about a the disk directly form the file handle.
fn read_header(disk_file: &File) -> Result<DiskHeader, DiskError> {
    // Read in first the block directly

    // We need to read a block before we have an actual disk, so we need
    // to call this function directly as a workaround.
    // We do not ignore the CRC here, because a corrupt CRC is a corrupt header.
    let header_block = super::io::read::read_block_direct(disk_file, 0, false)?;

    DiskHeader::extract_header(&header_block)
}

/// Abstraction for opening disks to allow easier debugging disks
/// The resulting header can be completely ignored to just get a raw file from the drive with no checks.
fn open_numbered(disk_number: u16, ignore_header: bool) -> Result<Disk, DiskError> {
    // Get path to the disk
    let disk_file = get_disk_file(disk_number)?;

    // Get the header
    // If we are ignoring the header, we will create a blank one.
    // It is the callers responsibility to ignore this header. The data will obviously be invalid.

    let header: DiskHeader = if ignore_header {
        // We will make our own header with spoofed data
        // Set everything to max values, or fill with 1's
        DiskHeader::spoof()
    } else {
        // Get the header normally
        read_header(&disk_file)?
    };

    // Assemble the disk!
    Ok(Disk {
        number: header.disk_number,
        header,
        disk_file,
    })
}

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
    let flags: HeaderFlags = HeaderFlags::empty();

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


/// Wipes a disk, destroying all data contained on it.
fn wipe(disk: &Disk) -> Result<(), DiskError> {
    // bye bye
    for i in 0..2880 {
        let result = disk.write_block(
            &RawBlock {
                block_index: i,
                data: [0u8; 512]
            }
        );
        // Make sure the block was wiped correctly
        if let Err(_) = result {
            // Writing failed.
            return Err(DiskError::WipeFailure)
        }
    }
    drop(disk); // gone.
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