// I think I slipped a disk.

// Imports

use crate::pool::disk::{
    blank_disk::blank_disk_struct::BlankDisk, unknown_disk::unknown_disk_struct::UnknownDisk,
};
use std::fs::File;

use enum_dispatch::enum_dispatch;
use thiserror::Error;

use crate::pool::disk::{
    generic::block::block_structs::RawBlock,
    pool_disk::pool_disk_struct::PoolDisk,
    standard_disk::standard_disk_struct::StandardDisk,
};

// Structs, Enums, Flags

/// The floppy drive
/// The FloppyDrive type doesn't contain anything itself, its just an interface for
/// retrieving the various types of disk.
pub struct FloppyDrive {
    // Nothing! This type is just for methods.
}

/// The different types of disks contained within a pool.
/// This contains disk info.
#[enum_dispatch]
#[derive(Debug)]
pub enum DiskType {
    Pool(PoolDisk),
    Standard(StandardDisk),
    Unknown(UnknownDisk),
    Blank(BlankDisk),
}

/// We also have another type that does not contain the disk info.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustDiskType {
    Pool,
    Standard,
    Unknown,
    Blank,
}

// Let us match the two
impl PartialEq<JustDiskType> for DiskType {
    fn eq(&self, other: &JustDiskType) -> bool {
        match self {
            DiskType::Pool(_) => matches!(other, JustDiskType::Pool),
            DiskType::Standard(_) => matches!(other, JustDiskType::Standard),
            DiskType::Unknown(_) => matches!(other, JustDiskType::Unknown),
            DiskType::Blank(_) => matches!(other, JustDiskType::Blank),
        }
    }
}

/// All disk types need to be able to create themselves from a raw block.
/// Or, be able to create themselves from a blank disk.
/// We also need to create fake disks to allow creating disks (confusing eh?)
pub trait DiskBootstrap {
    // TODO: Let disk bootstraps fail.
    /// Create brand new disk.
    /// This takes in a blank floppy disk, and does all the needed setup on the disk,
    /// such as writing the header, and other block setup.
    fn bootstrap(file: File, disk_number: u16) -> Result<Self, FloppyDriveError>
    where
        Self: std::marker::Sized;
    /// Create self from incoming header block and file.
    fn from_header(block: RawBlock, file: File) -> Self;
}
