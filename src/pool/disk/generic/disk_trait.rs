// All types of disk MUST implement this.
// TODO: Enforce that somehow.

use std::fs::File;

use enum_dispatch::enum_dispatch;

use crate::pool::disk::{
    drive_struct::DiskType,
    generic::block::block_structs::{BlockError, RawBlock},
};

use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::dense_disk::dense_disk_struct::DenseDisk;
use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;
use crate::pool::disk::unknown_disk::unknown_disk_struct::UnknownDisk;

// Generic disks must also have disk numbers, and be able to retrieve their inner File.
#[enum_dispatch(DiskType)] // Force every disk type to implement these methods.
pub trait GenericDiskMethods {
    /// Read a block
    /// Cannot bypass CRC.
    fn unchecked_read_block(&self, block_number: u16) -> Result<RawBlock, BlockError>;

    /// Write a block.
    fn unchecked_write_block(&mut self, block: &RawBlock) -> Result<(), BlockError>;

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
    fn flush(&mut self) -> Result<(), BlockError>;
}