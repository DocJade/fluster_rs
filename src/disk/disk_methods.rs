// TODO: safety
// TODO: the names of the functions are kinda hard to keep straight, need a better naming scheme.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek},
    os::windows::fs::{FileExt, OpenOptionsExt},
    path::Path, u16,
};

use crate::disk::{block::{block_structs::RawBlock, header::header_struct::{DiskHeader, HeaderFlags}}, disk_struct::{Disk, DiskError}};

// !! Only numbered options should be public! !!

impl Disk {
    //
    //  Private functions
    //

    // Open the disk currently connected to the system
    // Ensures the disk opened matches the provided ID.
    fn open_numbered(disk_number: u16) -> Result<Disk, DiskError> {
        open_numbered(disk_number)
    }

    // Used to initlize the data on a disk, not public.
    fn initialize_numbered(disk_file: &File, disk_number: u16) -> Result<(), DiskError> {
        initialize_numbered(disk_file, disk_number)
    }

    //
    //  Public functions
    //

    /// Opens the specified disk.
    /// Does not check if correct disk is in the drive.
    pub fn open(disk_number: u16) -> Result<Disk, DiskError> {
        open_numbered(disk_number)
    }

    /// Waits for user to insert the specified
    /// Will not return until specified disk is inserted, or if there are errors with the inserted disk.
    pub fn prompt_for_disk(disk_number: u16) -> Result<Disk, DiskError> {
        prompt_for_disk(disk_number)
    }

    /// Create a new disk. Destroys all data.
    /// Will not wipe disks that contain the magic.
    pub fn create(disk_number: u16) -> Result<(), DiskError> {
        create(disk_number)
    }

    /// Destroys ALL data on a disk.
    pub fn full_wipe(self) {
        full_wipe(self)
    }

    /// Destroys the header.
    pub fn wipe(self) {
        wipe(self)
    }
}

//
// Public functions
//



/// Prompt user to insert the disk we want.
/// If the disk is already in the drive, no prompt will happen.
/// Will error out for non-wrong disk related issues.
fn prompt_for_disk(disk_number: u16) -> Result<Disk, DiskError> {
    let mut is_user_an_idiot: bool = false; // Did the user put in the wrong disk when asked?
    let mut disk: Result<Disk, DiskError>;
    loop {
        // Try opening the current disk
        disk = open_numbered(disk_number);
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
///
/// This will only work on a disk that is blank / header-less.
/// This will create a disk of any disk number, it is up to the caller to ensure that
/// duplicate disks are not created, and to track the creation of this new disk.
fn create(disk_number: u16) -> Result<(), DiskError> {
    // Get the current disk
    let new_disk = get_disk_file(0).unwrap();
    
    // Now give it some head    er
    initialize_numbered(&new_disk, disk_number)

    // done
}

/// Wipes a disk, destroying all data contained on it.
fn full_wipe(disk: Disk) {
    // bye bye
    for i in 0..2880 {
        disk.write_block(
            RawBlock {
                block_index: Some(i),
                data: [0u8; 512]
            }
        );
    }
    drop(disk); // gone.
}

/// Wipe just the header.
fn wipe(disk: Disk) {
    // bye bye
    for i in 0..2 {
        disk.write_block(
            RawBlock {
                block_index: Some(i),
                data: [0u8; 512]
            }
        );
    }
    drop(disk); // gone.
}


//
// Private functions
//

/// Get the path of the disk
fn get_disk_file(disk_number: u16) -> Result<File, DiskError> {
    // If someone wants to port this to another operating system, this function will need appropriate changes
    // to remove its dependency on getting the raw floppy device from Windows.

    // TODO: Prevent blocking (Return NoDiskInserted if file does not load in under 1 second.)

    // If we are running with debug enabled, we are going to use a temp folder instead of the actual disk to speed up
    // development, waiting for disk seeks is slow and loud lol.

    if cfg!(debug_assertions) {
        println!("Debug mode on, opening a virtual disk.");
        // Get the tempfile.
        // These files do not delete themselves.

        // if disk 0 is missing, we need to make it.
        let _ = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open("./temp_disks/disk0.fsr").unwrap();

        // If the tempfile does not exist, that means `create` was never called, which is an issue.
        // This should never be allowed, so an unwrap is okay in this case.

        let temp_disk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .truncate(false)
            .open(format!("./temp_disks/disk{}.fsr", disk_number)).unwrap();

        // Make sure the file is one floppy big, should have no effect on pre-existing files, since
        // they will already be this size.
        temp_disk_file.set_len(512 * 2880).unwrap(); 

        return Ok(temp_disk_file);
    };

    // ==============================
    // == !!! POSSIBLE DANGER !!! ==
    // ==============================
    // We will be assuming that there is only one floppy disk, and it is always located in
    // the A: drive.

    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0) // Only I can have the file open.
        .open(Path::new(r"\\.\A:"))
        .unwrap())
}

/// Look for the magic "Fluster!" string.
fn check_for_magic(block_bytes: &[u8; 512]) -> bool {
    // is the "Fluster!" magic present?
    block_bytes[0..8] == *"Fluster!".as_bytes()
}

/// Returns header information about a the disk directly form the file handle.
fn read_header(disk_file: &File) -> Result<DiskHeader, DiskError> {
    // Read in first the block directly

    // We need to read a block before we have an actual disk, so we need
    // to call this function directly as a workaround.
    let header_block = super::io::read::read_block_direct(disk_file, 0);

    DiskHeader::extract_header(&header_block)
}

/// Abstraction for opening disks to allow easier debugging disks
fn open_numbered(disk_number: u16) -> Result<Disk, DiskError> {
    // Get path to the disk
    let disk_file = get_disk_file(disk_number)?;

    // get the header
    let header = read_header(&disk_file)?;

    // Assemble the disk!
    Ok(Disk {
        number: header.disk_number,
        header,
        disk_file,
    })
}

/// Initialize a disk for usage
///
/// Will not open uninitialized disks.
// TODO: Somehow prevent duplicate disk numbers?
fn initialize_numbered(mut disk_file: &File, disk_number: u16) -> Result<(), DiskError> {
    // A new, fresh disk!

    // Read in first the block
    let mut header_block: [u8; 512] = [0u8; 512];
    disk_file.seek(std::io::SeekFrom::Start(0)).unwrap();
    disk_file.read_exact(&mut header_block).unwrap();

    // Sanity check, make sure the disk isn't already initialized, we dont want to
    // lose data.

    if check_for_magic(&header_block) {
        // We can't re-initialize a disk with data on it.
        return Err(DiskError::NotBlank);
    }

    // Just in case the disk isn't totally blank, we need to wipe any blocks that aren't
    // available for allocation (ie, the header block, and the inode block).
    // Yes we are about to write over all of that data anyways, but better safe than sorry.
    // It would suck to start using some reserved space, just to find junk in there and crash.

    // Wipe the header and inode blocks
    disk_file.seek_write(&[0u8; 512 * 2], 0).unwrap();

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
        highest_known_disk: 0, // All disks have 0 in this position, except for the root which we will update.
        block_usage_map,
    };

    // Now serialize that, and write it
    let header_block = &header.to_disk_block();
    
    // Use the disk interface to write it safely
    super::io::write::write_block_direct(disk_file, header_block);

    // All done!
    Ok(())
}

// Wipes the disk
fn wipe_numbered(disk_file: &File, disk_number: u16) -> Result<(), DiskError> {
    todo!()
}