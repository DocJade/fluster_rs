// Writing files.

// We will take in InodeFile(s) instead of Extent related types, since we need info about how big files are so they are easier to extend.
// Creating files is handles on the directory side, since new files just have a name and location.

use crate::pool::disk::{drive_struct::FloppyDriveError, standard_disk::block::inode::inode_struct::InodeFile};

impl InodeFile {
    /// Update the contents of a file starting at the provided seek point.
    /// Will automatically grow file if needed.
    /// 
    /// Returns number of bytes written.
    fn write(self, bytes: Vec<u8>, seek_point: u64) -> Result<u64, FloppyDriveError> {
        todo!();
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