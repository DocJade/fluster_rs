
// TODO: safety

use std::{fs::{File, OpenOptions}, io::{Read, Seek}, os::windows::fs::{FileExt, OpenOptionsExt}, path::Path};

use crate::{block::header::header_struct::{DiskHeader, HeaderFlags}, disk::disk_struct::{Disk, DiskError}};

impl Disk {
    // Open the disk currently connected to the system
    pub fn open() -> Result<Disk, DiskError> {
        open()
    }
    pub fn initialize(disk_number: u16) -> Result<(), DiskError> {
        initialize(disk_number)
    }
}

// Functions

/// Get the path of the disk
/// 
/// This is here in case someone wants to port this to another os at some point i guess.
fn get_disk_file() -> File {

    // TODO: What happens if we open it multiple times?
    // TODO: That could cause issues, so we should prevent it.

    // ==============================
    // == !!! POSSIBLE DANGER !!! ==
    // ==============================
    // We will be assuming that there is only one floppy disk, and it is always located in
    // the A: drive.

    OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0) // Only I can have the file open.
        .open(Path::new(r"\\.\A:"))
        .unwrap()
}

/// Look for the magic "Fluster!" string.
fn check_for_magic(block_bytes: &[u8; 512]) -> bool {
    // is the "Fluster!" magic present?
    block_bytes[0..8] == *"Fluster!".as_bytes()
}

/// Open the disk currently connected to the system
/// 
/// Will not open uninitialized disks.
fn open() -> Result<Disk, DiskError> {
    let mut disk_file: File = get_disk_file();

    // Read in first the block
    let mut header_block: [u8; 512] = [0u8; 512];
    disk_file.seek(std::io::SeekFrom::Start(0)).unwrap();
    disk_file.read_exact(&mut header_block).unwrap();

    
    // Check that the disk has been initialized properly
    if !check_for_magic(&header_block) {
        // Missing magic. We cannot open an unformatted disk.
        return Err(DiskError::Uninitialized)
    }

    // Extract the header information
    let header: DiskHeader = DiskHeader::extract_header(header_block)?;

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
fn initialize(disk_number: u16) -> Result<(), DiskError> {
    // A new, fresh disk!
    let mut disk_file: File = get_disk_file();

    // Read in first the block
    let mut header_block: [u8; 512] = [0u8; 512];
    disk_file.seek(std::io::SeekFrom::Start(0)).unwrap();
    disk_file.read_exact(&mut header_block).unwrap();

    // Sanity check, make sure the disk isn't already initialized, we dont want to
    // lose data.

    if check_for_magic(&header_block) {
        // We can't re-initialize a disk with data on it.
        return Err(DiskError::NotBlank)
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
    disk_file.seek_write(
        &header.to_disk_block(),
        0
    ).unwrap();

    // All done!
    Ok(())
}