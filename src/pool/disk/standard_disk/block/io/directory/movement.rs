// Helpers to move between directories

use log::info;

use crate::pool::disk::{
    drive_struct::{FloppyDrive, FloppyDriveError, JustDiskType},
    generic::{generic_structs::pointer_struct::DiskPointer, io::cache::cache_io::CachedBlockIO},
    standard_disk::block::{
        directory::directory_struct::DirectoryBlock, inode::inode_struct::InodeBlock,
        io::directory::types::NamedItem,
    },
};

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
    pub fn change_directory(
        self,
        directory_name: String,
        return_to: Option<u16>,
    ) -> Result<Option<DirectoryBlock>, FloppyDriveError> {
        info!("Attempting to CD to `{directory_name}`");
        // Get all items in this directory

        let found_dir = self.find_item(&NamedItem::Directory(directory_name), return_to)?;
        if found_dir.is_none() {
            // The directory did not exist.
            info!("Directory did not exist.");
            return Ok(None)
        }
        info!("Directory exists.");
        let wanted = found_dir.expect("Just checked");

        // Directory exists, time to open that bad boy
        // Extract the location
        let final_destination = &wanted.location;
        info!(
            "Directory claims to live at: disk {} block {} offset {}",
            final_destination.disk.expect("Listing sets disk."),
            final_destination.block,
            final_destination.offset
        );
        // Since we got these items from self.list, all of these inode locations MUST have a disk destination
        // already set for us. So we dont have to check.

        // Load!
        // Now this doesn't point to the next directory block, it points to the next _Inode_ block
        // that points to it.
        let pointer: DiskPointer = DiskPointer {
            disk: final_destination.disk.expect("Listing sets disk."),
            block: final_destination.block,
        };

        let inode_block = InodeBlock::from_block(&CachedBlockIO::read_block(pointer, JustDiskType::Standard)?);

        // Now read in the inode
        let inode = inode_block
            .try_read_inode(final_destination.offset)
            .expect("Directories in a DirectoryBlock should point to a valid inode!");

        // Where is the block?
        let actual_next_block = inode
            .directory
            .expect("Should point to a directory inode, not a file.")
            .pointer;
        assert!(!actual_next_block.no_destination()); // Just in case...

        // Go go go!
        let new_dir_block: DirectoryBlock =
            DirectoryBlock::from_block(&CachedBlockIO::read_block(actual_next_block, JustDiskType::Standard)?);

        // Return to a disk if we need to
        if let Some(number) = return_to {
            let _ = FloppyDrive::open(number)?;
        }

        // All done! Enjoy the new block.
        Ok(Some(new_dir_block))
    }
}
