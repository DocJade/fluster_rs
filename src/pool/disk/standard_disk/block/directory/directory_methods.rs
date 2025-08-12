// Directory? Is that come kind of surgery?

// imports

// Implementations

use log::debug;

use crate::pool::disk::{
    drive_struct::FloppyDriveError, generic::{
        block::{block_structs::RawBlock, crc::add_crc_to_block},
        generic_structs::pointer_struct::DiskPointer, io::cache::cache_io::CachedBlockIO,
    }, standard_disk::block::{
        directory::directory_struct::{
            DirectoryBlock, DirectoryBlockError, DirectoryBlockFlags, DirectoryItemFlags, DirectoryItem,
        },
        inode::inode_struct::{Inode, InodeDirectory, InodeLocation, InodeTimestamp},
    }
};

// We can convert from a raw block to a directory bock, but not the other way around.
impl From<RawBlock> for DirectoryBlock {
    fn from(block: RawBlock) -> Self {
        Self::from_block(&block)
    }
}

impl DirectoryBlock {
    /// This assumes that you are writing this block back to the same
    /// location you got it from. If that is not the case, you need to swap out
    /// the origin BEFORE using this method.
    pub fn to_block(&self) -> RawBlock {
        directory_block_to_bytes(self)
    }
    pub fn from_block(block: &RawBlock) -> Self {
        directory_block_from_bytes(block)
    }

    /// Try to add an DirectoryItem to this block.
    ///
    /// Returns nothing.
    pub fn try_add_item(&mut self, item: &DirectoryItem) -> Result<(), DirectoryBlockError> {
        directory_block_try_add_item(self, item)
    }

    /// Try to remove a item from a directory.
    /// The item on the directory must match the item provided exactly.
    ///
    /// Returns nothing.
    pub(in super::super) fn try_remove_item(
        &mut self,
        item: &DirectoryItem,
    ) -> Result<(), DirectoryBlockError> {
        directory_block_try_remove_item(self, item)
    }

    /// Create a new directory block.
    /// 
    /// Requires the location/destination of this block.
    ///
    /// New directory blocks are the new final block on the disk.
    /// New directory blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block if needed.
    pub fn new(origin: DiskPointer) -> Self {
        new_directory_block(origin)
    }

    /// Get the items located within this block.
    /// This function is just to obscure the items by default, so higher up callers
    /// use higher abstractions
    pub fn get_items(&self) -> Vec<DirectoryItem> {
        self.directory_items.clone()
    }

    /// Check if this block is empty
    pub fn is_empty(&self) -> Result<bool, FloppyDriveError> {
        Ok(self.list()?.len() == 0)
    }
}

// funtions for those impls

fn directory_block_try_remove_item(
    block: &mut DirectoryBlock,
    incoming_item: &DirectoryItem,
) -> Result<(), DirectoryBlockError> {
    // Attempt to remove an item

    // attempt the removal
    if let Some(index) = block
        .directory_items
        .iter()
        .position(|item| item == incoming_item)
    {
        // Item exists.
        // update the free bytes counter
        block.bytes_free += incoming_item.to_bytes(block.block_origin.disk).len() as u16;

        // We can use swap_remove here since the ordering of items does not matter.
        let _ = block.directory_items.swap_remove(index);
        Ok(())
    } else {
        Err(DirectoryBlockError::NoSuchItem)
    }
}

fn directory_block_try_add_item(
    block: &mut DirectoryBlock,
    item: &DirectoryItem,
) -> Result<(), DirectoryBlockError> {
    // Attempt to add a new item to the directory.

    // check if we have room
    let new_item_bytes: Vec<u8> = item.to_bytes(block.block_origin.disk);
    let new_item_length: usize = new_item_bytes.len();

    if new_item_length > block.bytes_free.into() {
        // We don't have room for this inode. The caller will have to use another block.
        return Err(DirectoryBlockError::NotEnoughSpace);
    }

    // luckily since directory blocks dont require any ordering, we can just append it to the vec and update
    // the amount of free space remaining, since writing the actual data will just happen at the deserialization stage.

    block.directory_items.push(item.clone());

    // Update free space
    // This cast is fine, item lengths could never hit > 2^16
    block.bytes_free -= new_item_length as u16;

    // Done!
    Ok(())
}

fn new_directory_block(origin: DiskPointer) -> DirectoryBlock {
    // New block!

    // Flags
    // New blocks are assumed to be the last in the chain.
    let flags: DirectoryBlockFlags = DirectoryBlockFlags::empty(); // Currently unused.

    // Bytes free
    // An empty block has 501 bytes free.
    let bytes_free: u16 = 501;

    // Next block
    // New blocks assume we are the final block in the chain.
    let next_block: DiskPointer = DiskPointer::new_final_pointer();

    // Items
    // New blocks have no items. duh.
    // If this is the root disk, the caller needs to add the root directory.
    let directory_items: Vec<DirectoryItem> = Vec::new();

    // All done.
    DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        block_origin: origin,
        directory_items,
    }
}

/// We assume this is being written to the same place as it originated.
fn directory_block_to_bytes(block: &DirectoryBlock) -> RawBlock {
    // Deconstruct the bock
    let DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        #[allow(unused_variables)] // The items are extracted in a different way
        directory_items,
        block_origin,
    } = block;

    let mut buffer: [u8; 512] = [0u8; 512];

    // flags
    buffer[0] = flags.bits();

    // free bytes
    buffer[1..1 + 2].copy_from_slice(&bytes_free.to_le_bytes());

    // next block
    buffer[3..3 + 4].copy_from_slice(&next_block.to_bytes());

    // Directory items
    buffer[7..7 + 501].copy_from_slice(&block.item_bytes_from_vec(block_origin.disk));

    // add the CRC
    add_crc_to_block(&mut buffer);

    // All done!
    // This block is going to be written, thus does not need disk information.
    RawBlock {
        block_origin: *block_origin,
        data: buffer,
    }
}

fn directory_block_from_bytes(block: &RawBlock) -> DirectoryBlock {
    // Flags
    let flags: DirectoryBlockFlags = DirectoryBlockFlags::from_bits_retain(block.data[0]);

    // Free bytes, come and get 'em
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 = 2"));

    // Next block
    let next_block: DiskPointer =
        DiskPointer::from_bytes(block.data[3..3 + 4].try_into().expect("2 = 2"));

    // The directory items
    let directory_items: Vec<DirectoryItem> =
        DirectoryBlock::item_vec_from_bytes(&block.data[7..7 + 501], block.block_origin.disk);

    let block_origin = block.block_origin;

    // All done
    DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        block_origin,
        directory_items,
    }
}

// Conversions for the Vec of items
impl DirectoryBlock {
    fn item_bytes_from_vec(&self, destination_disk: u16) -> [u8; 501] {
        let mut index: usize = 0;
        let mut buffer: [u8; 501] = [0u8; 501];

        // Iterate over the items
        for i in &self.directory_items {
            // Cast item to bytes
            for byte in i.to_bytes(destination_disk) {
                // Put bytes in the buffer.
                buffer[index] = byte;
                index += 1;
            }
        }
        buffer
    }

    fn item_vec_from_bytes(bytes: &[u8], origin_disk: u16) -> Vec<DirectoryItem> {
        let mut items: Vec<DirectoryItem> = Vec::with_capacity(83); // Theoretical limit
        let mut index: usize = 0;
        loop {
            // Are we out of bytes?
            if index >= bytes.len() {
                break;
            }

            // Get the flags
            let flags: DirectoryItemFlags = DirectoryItemFlags::from_bits(bytes[index])
                .expect("Flags should only have used bits set.");

            // Check for marker bit
            if !flags.contains(DirectoryItemFlags::MarkerBit) {
                // No more items.
                break;
            }

            // Do the conversion
            let (item_size, item) = DirectoryItem::from_bytes(&bytes[index..], origin_disk);

            // increment index
            index += item_size as usize;

            // Done with this one
            items.push(item)
        }

        // All done
        items
    }
}

// Conversions for the Vec of items
impl DirectoryItem {
    /// Turn an item into bytes. Requires the destination disk.
    pub(super) fn to_bytes(&self, destination_disk: u16) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(262); // Theoretical limit
        // Flags
        vec.push(self.flags.bits());

        // Item name length
        vec.push(self.name_length);

        // The name of the item
        vec.extend(self.name.as_bytes());

        // location of the inode
        vec.extend(self.location.to_bytes(destination_disk));

        // All done
        vec
    }

    // Returns self, and how many bytes it took to construct this.
    pub(super) fn from_bytes(bytes: &[u8], origin_disk: u16) -> (u8, Self) {
        let mut index: usize = 0;
        // Flags
        let flags: DirectoryItemFlags =
            DirectoryItemFlags::from_bits(bytes[index]).expect("Flags should only have used bits set.");
        index += 1;

        // Make sure the flag is set
        assert!(flags.contains(DirectoryItemFlags::MarkerBit));

        // Item name length
        let name_length: u8 = bytes[index];
        index += 1;

        // Item name
        let name: String = String::from_utf8(bytes[index..index + name_length as usize].to_vec())
            .expect("File names should be valid UTF-8");
        index += name_length as usize;
        
        // Inode location
        let (location_size, location) = InodeLocation::from_bytes(&bytes[index..], origin_disk);
        index += location_size as usize;

        let done = Self {
            flags,
            name_length,
            name,
            location,
        };

        (index as u8, done)
    }

    /// Get the size of the item. Regardless of type.
    pub(crate) fn get_size(&self) -> Result<u64, FloppyDriveError> {
        debug!("Getting size of `{}`...", self.name);
        // Grab the inode to work with
        let inode: Inode = self.get_inode()?;

        // If this is a file, it's easy
        if let Some(file) = inode.extract_file() {
            debug!("Item is a file, getting size directly...");
            return Ok(file.get_size())
        }
        
        // Otherwise, this must be a directory, so we need the directory block
        debug!("Item is a directory...");
        let inode_directory: InodeDirectory = inode.extract_directory().expect("Guard.");
        
        // Load the block
        debug!("Getting origin block...");
        let raw_block: RawBlock = CachedBlockIO::read_block(inode_directory.pointer)?;
        
        let directory: DirectoryBlock = DirectoryBlock::from_block(&raw_block);
        
        // Now we can call the size method.
        debug!("Calling `get_size` on loaded DirectoryBlock...");
        directory.get_size()
    }

    /// Get when the inode / item was created.
    pub(crate) fn get_created_time(&self) -> Result<InodeTimestamp, FloppyDriveError> {
        // get the inode
        let inode = self.get_inode()?;
        Ok(inode.created)
    }

    /// Get when the inode / item was modified.
    pub(crate) fn get_modified_time(&self) -> Result<InodeTimestamp, FloppyDriveError> {
        // get the inode
        let inode = self.get_inode()?;
        Ok(inode.modified)
    }

    /// All item types point to a block that holds their information.
    /// You can see what block they point to, but you REALLY should not be doing reads like this.
    fn get_items_pointer(&self) -> Result<DiskPointer, FloppyDriveError> {
        Ok(self.get_inode()?.get_pointer())
    }

    /// Turn a directory type DirectoryItem into a DirectoryBlock.
    /// 
    /// Panics if fed a file.
    pub(crate) fn get_directory_block(&self) -> Result<DirectoryBlock, FloppyDriveError> {
        // Grab the inode to work with
        let inode: Inode = self.get_inode()?;

        if let Some(dir) = inode.extract_directory() {
            // Get the directory block
            let raw_block: RawBlock = CachedBlockIO::read_block(dir.pointer)?;
            Ok(DirectoryBlock::from_block(&raw_block))
        } else {
            // This was not a file.
            panic!("Attempted to turn a DirectoryItem of File type into a DirectoryBlock!")
        }
    }
}
