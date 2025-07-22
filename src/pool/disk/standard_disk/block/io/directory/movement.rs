// Helpers to move between directories

use log::{debug, info};

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::io::checked_io::CheckedIO, standard_disk::block::{directory::directory_struct::DirectoryBlock, inode::inode_struct::InodeBlock, io::directory::types::NamedItem}};

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
    pub fn change_directory(self, directory_name: String, return_to: Option<u16>) -> Result<Option<DirectoryBlock>, FloppyDriveError> {
        info!("Attempting to CD to `{directory_name}`");
        // Get all items in this directory
        let items = self.list(return_to)?;
        // Is it in there?
        let wanted = if let Some(item) = NamedItem::Directory(directory_name.clone()).find_in(items.as_slice()) {
            // there it is!
            info!("Directory exists.");
            item
        } else {
            // Directory does not exist.
            info!("Directory did not exist.");
            // Temporary TODO:
            let copy_bc_vscode = directory_name.as_str();
            panic!();
            return Ok(None);
        };
        
        // Directory exists, time to open that bad boy
        // Extract the location
        let final_destination = &wanted.location;
        info!("Directory claims to live at: disk {} block {} offset {}",
            final_destination.disk.expect("Listing sets disk."),
            final_destination.block,
            final_destination.offset
        );
        // Since we got these items from self.list, all of these inode locations MUST have a disk destination
        // already set for us. So we dont have to check.

        // Load!
        let disk = match FloppyDrive::open(final_destination.disk.expect("self.list should set the disk."))? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Directory inode locations should NEVER point to a non-standard disk."),
        };

        // Now this doesn't point to the next directory block, it points to the next _Inode_ block
        // that points to it.
        let inode_block = InodeBlock::from_block(&disk.checked_read(final_destination.block)?);

        // Now read in the inode
        let inode = inode_block.try_read_inode(final_destination.offset).expect("Directories in a DirectoryBlock should point to a valid inode!");
        
        // Where is the block?
        let actual_next_block = inode.directory.expect("Should point to a directory inode, not a file.").pointer;
        assert!(!actual_next_block.no_destination()); // Just in case...

        // Go go go!
        let block_disk = match FloppyDrive::open(actual_next_block.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Directory inodes should point to standard disks."),
        };
        let new_dir_block: DirectoryBlock = DirectoryBlock::from_block(&block_disk.checked_read(actual_next_block.block)?);
        
        // Return to a disk if we need to
        if let Some(number) = return_to {
            let _ = FloppyDrive::open(number)?;
        }
        
        // All done! Enjoy the new block.
        Ok(Some(new_dir_block))
    }
}