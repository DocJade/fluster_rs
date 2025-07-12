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

    // Wipes the disk
    fn wipe_numbered(disk_file: &File, disk_number: u16) -> Result<(), DiskError> {
        wipe_numbered(disk_file, disk_number)
    }

    //
    //  Public functions
    //

    /// Opens the specified disk.
    pub fn open(disk_number: u16) -> Result<Disk, DiskError> {
        open_numbered(disk_number)
    }

    /// Writes the header for a disk. Does NOT destroy data.
    pub fn initialize(disk_number: u16) -> Result<(), DiskError> {
        initialize(disk_number)
    }

    /// Destroys ALL data on a disk.
    pub fn wipe(disk_number: u16) -> Result<(), DiskError> {
        wipe(disk_number)
    }
}

//
// Public functions
//

/// Opens a numbered disk
///
/// If the disk is not inserted, the user will be prompted to insert the disk.
fn open(disk_number: u16) -> Result<Disk, DiskError> {
    // Get the current disk

    todo!();
}

/// Initializes a disk by writing header data.
///
/// This will only work on a disk that is blank / header-less.
fn initialize(disk_number: u16) -> Result<(), DiskError> {
    // Get the current disk

    todo!();
}

/// Wipes a disk, destroying all data contained on it.
///
/// This will only work on a disk that is blank / header-less.
fn wipe(disk_number: u16) -> Result<(), DiskError> {
    // Get the current disk

    todo!();
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
        // Get the tempfile, or make it if it does not exist.
        // These files do not delete themselves.

        let temp_disk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(format!("./temp_disks/disk{}.fsr", disk_number))
            .unwrap();

        // Make sure the file is one floppy big, should have no effect on pre-existing files, since
        // they will already be this size.
        temp_disk_file.set_len(512 * 2880).unwrap(); 

        

        // check if we've already created this disk
        let check_header = read_header(&temp_disk_file);
        if check_header.as_ref().is_err_and(|x| *x == DiskError::Uninitialized) {
            // its a new disk, we must create it.
            println!("Disk did not exist yet. Initializing it...");
            initialize_numbered(&temp_disk_file, disk_number)?;
            return Ok(temp_disk_file);
        } else if check_header.is_err() {
            // Well, something went wrong
            panic!("Failed to get header for temporary disk!")
        } else {
            // File was already there, we can return the file as is.
            return Ok(temp_disk_file);
        };
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

    Ok(DiskHeader::extract_header(&header_block)?)
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
        block_usage_map,
    };

    // Now serialize that, and write it
    disk_file.seek_write(&header.to_disk_block().unwrap().data, 0).unwrap();

    // All done!
    Ok(())
}

// Wipes the disk
fn wipe_numbered(disk_file: &File, disk_number: u16) -> Result<(), DiskError> {
    todo!()
}