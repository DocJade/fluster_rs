// Reading a block is way easier than writing it.

use log::{debug, trace};

use crate::pool::disk::{drive_struct::{DiskType, FloppyDrive, FloppyDriveError}, generic::{generic_structs::pointer_struct::DiskPointer, io::checked_io::CheckedIO}, standard_disk::block::{file_extents::file_extents_struct::{FileExtent, FileExtentBlock}, inode::inode_struct::{InodeBlock, InodeFile}}};

impl InodeFile {
    // Local functions
    /// Extract all of the extents and spit out a list of all of the blocks.
    pub(super) fn to_pointers(&self, return_to: Option<u16>) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        go_to_pointers(self, return_to)
    }
    /// Extract all of the extents.
    /// 
    /// Optionally returns to provided disk.
    pub(super) fn to_extents(&self, return_to: Option<u16>) -> Result<Vec<FileExtent>, FloppyDriveError> {
        let root = self.get_root_block()?;
        go_to_extents(&root, return_to)
    }
    /// Goes and gets the FileExtentBlock this refers to.
    fn get_root_block(&self) -> Result<FileExtentBlock, FloppyDriveError> {
        go_get_root_block(self)
    }
}



fn go_to_pointers(location: &InodeFile, return_to: Option<u16>) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    // get extents
    let extents = location.to_extents(return_to)?;
    // Extract all the blocks
    let mut blocks: Vec<DiskPointer> = Vec::new();

    // For each extent
    for e in extents {
        // each block that the extent references
        for n in 0..e.length {
            blocks.push(DiskPointer {
                disk: e.disk_number.expect("Read extents should have disk"),
                block: e.start_block + n as u16
            });
        }
    }

    Ok(blocks)
}

// Functions

fn go_to_extents(
    block: &FileExtentBlock,
    return_to: Option<u16>,
) -> Result<Vec<FileExtent>, FloppyDriveError> {
    // Totally didn't just lift the directory logic and tweak it, no sir.
    debug!("Extracting extents for a file...");
    // We need to iterate over the entire ExtentBlock chain and get every single item.
    // We assume we are handed the first ExtentBlock in the chain.
    let mut extents_found: Vec<FileExtent> = Vec::new();
    let mut current_dir_block: FileExtentBlock = block.clone();
    // To keep track of what disk an extent is from
    let mut current_disk: u16 = block.block_origin.disk;

    // Big 'ol loop, we will break when we hit the end of the directory chain.
    loop {
        // Add all of the contents of the current directory to the total
        // But we will add the disk location data to these structs, it is the responsibility of the caller
        // to remove these disk locations if they no longer need them.
        // Otherwise if we didn't add the disk location for every item, it would be impossible
        // to know where a local pointer goes.
        let mut new_items = current_dir_block.get_extents();
        for item in &mut new_items {
            // If the disk location is already there, we wont do anything.
            if item.disk_number.is_none() {
                // There was no disk information, it must be local.
                item.disk_number = Some(current_disk)
            }
            // Otherwise there was already a disk being pointed to.
            // Overwriting it here would corrupt it.
        }

        extents_found.extend_from_slice(&new_items);

        // I want to get off Mr. Bone's wild ride
        if current_dir_block.next_block.no_destination() {
            // We're done!
            trace!("Done getting FileExtent(s).");
            break;
        }

        trace!("Need to continue on the next block.");
        // Time to load in the next block.
        let next_block = current_dir_block.next_block;

        // Update what disk we're on
        current_disk = next_block.disk;

        let disk = match FloppyDrive::open(next_block.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Why did the block point to a non-standard disk?"),
        };

        current_dir_block = FileExtentBlock::from_block(&disk.checked_read(next_block.block)?);

        // Onwards!
        continue;
    }

    // We will not sort this vec, since the order matters. The blocks are added to extend the file always at the end.
    // TODO: Assert that this is true ^

    // Return to a specified block if the caller requested it
    if let Some(number) = return_to {
        _ = FloppyDrive::open(number)?;
    }

    Ok(extents_found)
}


fn go_get_root_block(file: &InodeFile) -> Result<FileExtentBlock, FloppyDriveError> {

    // Make sure this actually goes somewhere
    assert!(!file.pointer.no_destination());

    let disk = match FloppyDrive::open(file.pointer.disk)? {
            DiskType::Standard(standard_disk) => standard_disk,
            _ => unreachable!("Why did the block point to a non-standard disk?"),
        };
    let block = FileExtentBlock::from_block(&disk.checked_read(file.pointer.block)?);
    Ok(block)
}