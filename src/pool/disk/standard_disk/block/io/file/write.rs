// Writing files.

// We will take in InodeFile(s) instead of Extent related types, since we need info about how big files are so they are easier to extend.
// Creating files is handles on the directory side, since new files just have a name and location.

use crate::pool::disk::{drive_struct::{FloppyDrive, FloppyDriveError}, generic::{block::crc::add_crc_to_block, generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::block::{file_extents::{file_extents_methods::DATA_BLOCK_OVERHEAD, file_extents_struct::FileExtent}, inode::inode_struct::InodeFile}};

impl InodeFile {
    /// Update the contents of a file starting at the provided seek point.
    /// Will automatically grow file if needed.
    /// 
    /// Returns number of bytes written.
    fn write(self, bytes: &[u8], seek_point: u64) -> Result<u64, FloppyDriveError> {
       go_write(self, bytes, seek_point)
    }
    /// Deletes a file by deallocating every block the file used to take up, including
    /// all of the FileExtent blocks that were used to construct the file.
    fn delete(self) -> Result<(), FloppyDriveError> {
        todo!();
    }
    /// Truncates a file. Deallocates every block that used to hold data for this file.
    /// Does not delete the origin FileExtent block.
    fn truncate(&mut self) -> Result<(), FloppyDriveError> {
        todo!();
    }
}

fn go_write(inode: InodeFile, bytes: &[u8], seek_point: u64) -> Result<u64, FloppyDriveError> {
    // Decompose the file into its pointers
    // let blocks: Vec<DiskPointer> = inode.
    todo!();
}

/// Updates a block with new content, overwriting previous content at an offset.
/// 
/// You can feed in as many bytes as you like, but it will only write as many as it can.
/// 
/// Offset is the first data byte, not the first byte of the block!
/// 
/// Returns number of bytes written.
fn update_block(block: DiskPointer, bytes: &[u8], offset: u16) -> Result<usize, FloppyDriveError> {

    // How much data a block can hold
    let data_capacity = 512 - DATA_BLOCK_OVERHEAD as usize;
    let offset = offset as usize;

    // Check for impossible offsets
    assert!(offset < data_capacity, "Tried to write outside of the capacity of a block.");
    
    // Correctly calculate bytes to write based on REMAINING space.
    let remaining_space = data_capacity - offset;
    let bytes_to_write = std::cmp::min(bytes.len(), remaining_space);
    
    // We also don't support 0 byte writes.
    // Since that would be a failure mode of the caller, in theory could be
    // stuck in an infinite loop type shi.
    // Why panic? It won't if you fix the caller! :D
    assert_ne!(bytes_to_write, 0, "Tried to write 0 bytes to a block!");


    // load the block
    let mut disk = match FloppyDrive::open(block.disk)? {
        crate::pool::disk::drive_struct::DiskType::Standard(standard_disk) => standard_disk,
        _ => unreachable!("How are we reading a block from a non-standard disk?"),
    };
    let mut block_copy = disk.checked_read(block.block)?;
    
    // Modify that sucker
    // Skip the first byte with the flag
    let start = offset + 1;
    let end = start + bytes_to_write;

    block_copy.data[start..end].copy_from_slice(&bytes[..bytes_to_write]);

    // Update the crc
    add_crc_to_block(&mut block_copy.data);

    // Write that sucker
    disk.checked_update(&block_copy)?;

    // Return the number of bytes we wrote.
    Ok(bytes_to_write)
}