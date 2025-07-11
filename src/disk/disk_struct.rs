// Information about a disk

use std::{fs::{File, OpenOptions}, io::{Read, Seek}, path::Path};

use crate::block::header::header_struct::DiskHeader;


pub struct Disk {
    // Which disk is this?
    pub number: u16,
    // The disk header
    pub header: DiskHeader,
    // The file that refers to this disk
    pub(super) disk_file: std::fs::File,
}
#[derive(Debug)]
pub enum DiskError {
    Uninitialized,
}

// TODO: safety

impl Disk {
    // Open the disk currently connected to the system
    pub fn open() -> Result<Disk, DiskError> {
        open()
    }
}

// Functions

/// Open the disk currently connected to the system
/// 
/// Will not open uninitialized disks.
fn open() -> Result<Disk, DiskError> {
    // !!POSSIBLE DANGER!!
    // We will be assuming that there is only one floppy disk, and it is always located in
    // the A: drive.
    let mut disk_file: File = OpenOptions::new()
        .read(true)
        .write(true)
        .open(Path::new(r"\\.\A:"))
        .unwrap();
    
    // Check that the disk has been initialized properly

    // Go to the start of the disk
    disk_file.seek(std::io::SeekFrom::Start(0));

    // is the "Fluster!" magic present?
    let mut tag_buffer: [u8; 8] = [0u8; 8];
    disk_file.read_exact(&mut tag_buffer);

    if tag_buffer != "Fluster!".as_bytes() {
        // Header is missing. We cannot open an uninitialized disk.
        return Err(DiskError::Uninitialized);
    }

    // Disk is initialized, we will now fill in the rest of the disk data.


    // Read in the rest of the block
    let mut header_block: [u8; 512] = [0u8; 512];
    disk_file.seek(std::io::SeekFrom::Start(0));
    disk_file.read_exact(&mut header_block);


    // What's this disk's number?
    let number: u16 = u16::from_le_bytes(
        header_block[9..9 + 2]
            .try_into()
            .expect("Impossible.")
    );

    // Extract the header information
    let header: DiskHeader = DiskHeader::extract_header(header_block);

    // Assemble the disk!
    Ok(Disk {
        number,
        header,
        disk_file,
    })

}