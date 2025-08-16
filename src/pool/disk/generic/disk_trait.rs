// All types of disk MUST implement this.
// TODO: Enforce that somehow.

use std::fs::File;

use enum_dispatch::enum_dispatch;

use crate::{
    error_types::drive::DriveError,
    pool::disk::{
        drive_struct::DiskType,
        generic::{
            block::block_structs::RawBlock,
            generic_structs::pointer_struct::DiskPointer
        },
    }
};

// Generic disks must also have disk numbers, and be able to retrieve their inner File.
#[enum_dispatch(DiskType)] // Force every disk type to implement these methods.
pub trait GenericDiskMethods {
    /// Read a block
    /// Cannot bypass CRC.
    fn unchecked_read_block(&self, block_number: u16) -> Result<RawBlock, DriveError>;

    /// Write a block.
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), DriveError>;

    /// Write chunked data, starting at a block.
    fn unchecked_write_large(&mut self, data: Vec<u8>, start_block: DiskPointer) -> Result<(), DriveError>;

    /// Get the inner file.
    fn disk_file(self) -> File;

    /// Get the inner file for write operations.
    fn disk_file_mut(&mut self) -> &mut File;

    /// Get the number of the floppy disk.
    fn get_disk_number(&self) -> u16;

    /// Set the number of this disk.
    fn set_disk_number(&mut self, disk_number: u16);

    /// Sync all in-memory information to disk
    /// Headers and such.
    fn flush(&mut self) -> Result<(), DriveError>;
}
