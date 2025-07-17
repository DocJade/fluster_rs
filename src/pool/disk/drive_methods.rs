// Methods that are generic across all types of disk.

// Imports

use crate::pool::disk::drive_struct::DiskType;

use super::drive_struct::FloppyDriveError;
use super::drive_struct::Disk;

// Implementations


/// Various operations on the underlying Disk.
/// This is meant to be high level, just enough to get to the disk type below.
impl FloppyDrive {
    /// Open the disk currently in the drive, regardless of disk type or disk number.
    fn open_direct() -> DiskType {
        open_and_deduce_disk()
    }
    
    /// Opens a specific disk, or waits until the user inserts that disk.
    fn open(disk_number: u16) -> DiskType {
        todo!()
    }
}




// Functions for implementations

fn open_and_deduce_disk() -> DiskType {
    todo!()
}