// Helpers to move between directories

use crate::pool::disk::{drive_struct::{FloppyDrive, FloppyDriveError}, standard_disk::block::{directory::directory_struct::DirectoryBlock, io::directory::types::NamedItem}};

impl DirectoryBlock {
    /// Attempts to open a directory in the current directory block.
    /// This will check if the directory already exists, if it doesn't, 
    /// Ok(None) will be returned, because there was no directory to open.
    /// 
    /// May swap disks, will end up on whatever disk the new directory is located on, unless
    /// you specify a return location.
    /// 
    /// If there is no new directory, this will end up wherever the end of the input directory was, unless
    /// you set the return disk.
    fn change_directory(self, directory_name: String, return_to: Option<u16>) -> Result<Option<DirectoryBlock>, FloppyDriveError> {
        // Get all items in this directory
        let items = self.list(return_to)?;
        // Is it in there?
        let index = match NamedItem::Directory(directory_name).find_in(&items) {
            Ok(ok) => ok,
            Err(_) => {
                // Directory does not exist.
                return Ok(None);
            },
        };
        // Directory exists, time to open that bad boy
        // Extract the location
        let final_destination = &items[index].location;
        
        // let new_dir = DirectoryBlock::from_block(FloppyDrive::)
        todo!();

    }
}