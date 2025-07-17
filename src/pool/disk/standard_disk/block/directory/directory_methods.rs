// Directory? Is that come kind of surgery?

// imports


// Implementations


// We can convert from a raw block to a directory bock, but not the other way around.
impl From<RawBlock> for DirectoryBlock {
    fn from(block: RawBlock) -> Self {
        Self::from_bytes(&block)
    }
}





impl DirectoryBlock {
    /// Block number must be known at creation time for safe writing.
    pub(super) fn to_bytes(&self, block_number: u16) -> RawBlock {
        directory_block_to_bytes(self, block_number)
    }
    pub(super) fn from_bytes(block: &RawBlock) -> Self {
        directory_block_from_bytes(block)
    }
    /// Try to add an DirectoryItem to this block.
    /// 
    /// Returns nothing.
    pub(super) fn try_add_item(&mut self, item: DirectoryItem) -> Result<(), DirectoryBlockError> {
        directory_block_try_add_item(self, item)
    }
    /// Try to remove a item from a directory.
    /// The item on the directory must match the item provided exactly.
    /// 
    /// Returns nothing.
    pub(super) fn try_remove_item(&mut self, item: DirectoryItem) -> Result<(), DirectoryBlockError> {
        directory_block_try_remove_item(self, item)
    }
    /// Create a new inode block
    /// 
    /// New directory blocks are the new final block on the disk.
    /// New directory blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block.
    pub(super) fn new() -> Self {
        new_directory_block()
    }
}

// funtions for those impls

fn directory_block_try_remove_item(block: &mut DirectoryBlock, incoming_item: DirectoryItem) -> Result<(), DirectoryBlockError> {
    // Attempt to remove an item

    // attempt the removal
    if let Some(index) = block.directory_items.iter().position(|item| *item == incoming_item) {
        // Item exists.
        // update the free bytes counter
        block.bytes_free += incoming_item.to_bytes().len() as u16;

        // We can use swap_remove here since the ordering of items does not matter.
        block.directory_items.swap_remove(index);
        Ok(())
    } else {
        Err(DirectoryBlockError::NoSuchItem)
    }
}

fn directory_block_try_add_item(block: &mut DirectoryBlock, item: DirectoryItem) -> Result<(), DirectoryBlockError> {
    // Attempt to add a new item to the directory

    // check if we have room
    let new_item_bytes: Vec<u8> = item.to_bytes();
    let new_item_length: usize = new_item_bytes.len();

    if new_item_length > block.bytes_free.into() {
        // We don't have room for this inode. The caller will have to use another block.
        return Err(DirectoryBlockError::NotEnoughSpace)
    }

    // luckily since directory blocks dont require any ordering, we can just append it to the vec and update
    // the amount of free space remaining, since writing the actual data will just happen at the deserialization stage.

    block.directory_items.push(item);

    // Update free space
    // This cast is fine, item lengths could never hit > 2^16
    block.bytes_free -= new_item_length as u16;

    // Done!
    Ok(())
}

fn new_directory_block() -> DirectoryBlock {
    // New block!

    // Flags
    // New blocks are assumed to be the last in the chain.
    let flags: DirectoryBlockFlags = DirectoryBlockFlags::FinalDirectoryBlockOnThisDisk;

    // Bytes free
    // An empty block has 503 bytes free.
    let bytes_free: u16 = 503;

    // Next block
    // New blocks assume we are the final block in the chain.
    let next_block: u16 = u16::MAX;

    // Items
    // New blocks have no items. duh.
    // If this is the root disk, the caller needs to add the root directory.
    let directory_items: Vec<DirectoryItem> = Vec::new();

    // All done.
    DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        directory_items,
    }
}

fn directory_block_to_bytes(block: &DirectoryBlock, block_number: u16) -> RawBlock {
    // Deconstruct the bock
    let DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        #[allow(unused_variables)] // The items are extracted in a different way
        directory_items,
    } = block;

    let mut buffer: [u8; 512] = [0u8; 512];


    // flags
    buffer[0] = flags.bits();

    // free bytes
    buffer[1..1 + 2].copy_from_slice(&bytes_free.to_le_bytes());

    // next block
    buffer[3..3 + 2].copy_from_slice(&next_block.to_le_bytes());

    // Directory items
    buffer[5..5 + 503].copy_from_slice(&block.item_bytes_from_vec());

    // add the CRC
    add_crc_to_block(&mut buffer);

    // All done!
    RawBlock {
        block_index: block_number,
        data: buffer
    }

}

fn directory_block_from_bytes(block: &RawBlock) -> DirectoryBlock {

    // Flags
    let flags: DirectoryBlockFlags = DirectoryBlockFlags::from_bits_retain(block.data[0]);

    // Free bytes, come and get 'em
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 = 2"));

    // Next block
    let next_block: u16 = u16::from_le_bytes(block.data[3..3 + 2].try_into().expect("2 = 2"));

    // The directory items
    let directory_items: Vec<DirectoryItem> = DirectoryBlock::item_vec_from_bytes(&block.data[5..5 + 503]);

    // All done
    DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        directory_items,
    }
}



// Conversions for the Vec of items
impl DirectoryBlock {
    fn item_bytes_from_vec(&self) -> [u8; 503] {
        let mut index: usize = 0;
        let mut buffer: [u8; 503] = [0u8; 503];

        for i in &self.directory_items {
            for byte in i.to_bytes() {
                buffer[index] = byte;
                index += 1;
            }
        }
        buffer
    }
    fn item_vec_from_bytes(bytes: &[u8]) -> Vec<DirectoryItem> {
        let mut items: Vec<DirectoryItem> = Vec::with_capacity(83); // Theoretical limit
        let mut index: usize = 0;
        loop {
            // Are we out of bytes?
            if index >= bytes.len() {
                break
            }

            
            // Get the flags
            let flags: DirectoryFlags = DirectoryFlags::from_bits(bytes[index]).expect("Flags should only have used bits set.");

            // Check for marker bit
            if !flags.contains(DirectoryFlags::MarkerBit) {
                // No more items.
                break
            }

            // Do the conversion
            let item: DirectoryItem = DirectoryItem::from_bytes(&bytes[index..]);

            // increment index
            index += item.to_bytes().len();
            
            // Done with this one
            items.push(item)
        }

        // All done
        items
    }
}

// Conversions for the Vec of items
impl DirectoryItem {
    pub(super) fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(262); // Theoretical limit
        // Flags
        vec.push(self.flags.bits());

        // Item name length
        vec.push(self.name_length);

        // The name of the item
        vec.extend(self.name.as_bytes());

        // location of the inode
        vec.extend(self.location.to_bytes());

        // All done
        vec

    }
    pub(super) fn from_bytes(bytes: &[u8]) -> Self {
        let mut index: usize = 0;
        // Flags
        let flags: DirectoryFlags = DirectoryFlags::from_bits(bytes[index]).expect("Flags should only have used bits set.");
        index += 1;
        
        // Item name length
        let name_length: u8 = bytes[index];
        index += 1;
        
        // Item name
        let name: String = String::from_utf8(bytes[index..index + name_length as usize].to_vec()).expect("File names should be valid UTF-8");
        index += name_length as usize;

        // inode location
        // must be fed either 3 or 5 bytes depending on type
        let location_length: usize = if flags.contains(DirectoryFlags::OnThisDisk) {
            // On this disk, so 3
            3
        }  else {
            5
        };

        let location: InodeLocation = InodeLocation::from_bytes(&bytes[index..index + location_length]);

        Self {
            flags,
            name_length,
            name,
            location,
        }
    }
}

impl InodeLocation {
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(5); // Max size of this type

        // Disk number
        if self.disk.is_some() {
            vec.extend_from_slice(&self.disk.expect("Already checked").to_le_bytes());
        }

        // Block on disk
        vec.extend_from_slice(&self.block.to_le_bytes());

        // index into the block
        vec.push(self.index);

        vec
    }
    /// Do not feed more than 5 bytes.
    fn from_bytes(bytes: &[u8]) -> Self {
        // Disk number
        let mut index: usize = 0;
        // we need to extract the disk number if length is 5
        let disk: Option<u16> = if bytes.len() == 5 {
            index += 2; // Offset by 2 bytes, since the next items are relative to this
            Some(u16::from_le_bytes(bytes[..2].try_into().expect("2 = 2")))
        } else {
            None
        };
        
        // Block on disk
        let block: u16 = u16::from_le_bytes(bytes[index..index + 2].try_into().expect("2 = 2"));
        index += 2;

        // Index into Inode block
        let index: u8 = bytes[index];

        Self {
            disk,
            block,
            index,
        }
    }
}