// Inode a block, then he moved away.

use crate::pool::disk::{block::{block_structs::RawBlock, crc::add_crc_to_block, inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeBlockFlags, InodeDirectory, InodeFile, InodeFlags, InodeReadError, InodeTimestamp}}, generic_structs::{find_space::{find_free_space, BytePingPong}, pointer_struct::DiskPointer}};

impl From<RawBlock> for InodeBlock {
    fn from(value: RawBlock) -> Self {
        from_raw_block(&value)
    }
}

// Add ability for inodes to have space searched for them
impl BytePingPong for Inode {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl InodeBlock {
    pub(super) fn to_bytes(&self) -> RawBlock {
        to_raw_bytes(self)
    }
    pub(super) fn from_bytes(block: &RawBlock) -> Self {
        from_raw_block(&block)
    }
    /// Create a new inode block
    /// 
    /// New Inode blocks are the new final block on the disk.
    /// New Inode blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block.
    pub fn new() -> Self {
        new_inode_block()
    }
    /// Try to add an Inode to this block.
    /// Updates the byte usage counter.
    /// 
    /// Returns the index of the added inode. (the first inode is 0)
    pub fn try_add_inode(&mut self, inode: Inode) -> Result<u16, InodeBlockError> {
        inode_block_try_add_inode(self, inode)
    }
    /// Removes inodes based off of the offset into the block. (NOT index!)
    /// Updates the byte usage counter.
    /// This does not remove the data the inode points to. The caller is responsible for propagation.
    /// 
    /// Returns nothing.
    pub fn try_remove_inode(&mut self, inode_offset: u16) -> Result<(), InodeBlockError> {
        inode_block_try_remove_inode(self, inode_offset)
    }
    /// Try and read an inode from the block.
    /// 
    /// Returns Inode.
    pub fn try_read_inode(&self, inode_offset: u16) -> Result<Inode, InodeReadError> {
        inode_block_try_read_inode(self, inode_offset)
    }
}

//
// Functions
//

fn inode_block_try_read_inode(block: &InodeBlock, offset: u16) -> Result<Inode, InodeReadError> {
    // Attempt to read in the inode at this location
    // extract function at bottom of file

    // Bounds checking
    if offset as usize > block.inodes_data.len() {
        // We cannot read past the end of the end of the data!
        return Err(InodeReadError::ImpossibleOffset);
    }
    // get a slice with that inode and deserialize it
    return Ok(Inode::from_bytes(&block.inodes_data[offset as usize..]));
}

fn inode_block_try_remove_inode(block: &mut InodeBlock, inode_offset: u16) -> Result<(), InodeBlockError> {
    // Attempt to remove an inode from the block

    // Assumption:
    // Caller gave us a valid offset.
    // There isn't a great way to check this besides scanning through the entire block to find all of the
    // inodes, but we can at least check the marker bit.
    // Additionally, if there are extra unused bits set in the flags, this is almost certainly an invalid offset.
    let flags = match InodeFlags::from_bits(block.inodes_data[inode_offset as usize])  {
        Some(ok) => ok,
        None => {
            // Unused bits are set. This cannot be the start of an inode.
            return Err(InodeBlockError::InvalidOffset);
        },
    };

    if !flags.contains(InodeFlags::MarkerBit) {
        // Missing flag.
        // This cannot be the beginning of an inode.
        return Err(InodeBlockError::InvalidOffset);
    };

    // Assumption: There is a valid inode at the provided offset
    // Yes the cast back and forth is silly, but at least its easy.
    let inode_to_remove_length: usize = Inode::from_bytes(&block.inodes_data[inode_offset as usize..]).to_bytes().len();

    // Blank out those bytes
    // This range is inclusive because we are removing the last byte of the item as well, not just up to the last byte.
    block.inodes_data[inode_offset as usize..inode_offset as usize + inode_to_remove_length].iter_mut().for_each(|byte| *byte = 0);

    // sanity check, bytes are now empty
    #[cfg(test)]
    {
        for i in 0..inode_to_remove_length {
            assert_eq!(block.inodes_data[inode_offset as usize + i], 0)
        }
    }
    

    // update how many bytes are free
    block.bytes_free += inode_to_remove_length as u16;

    // Done
    Ok(())
}

fn inode_block_try_add_inode(inode_block: &mut InodeBlock, new_inode: Inode) -> Result<u16, InodeBlockError> {

    // Attempt to add an inode to the block.

    // Check if we have room for the new inode.
    let new_inode_bytes: Vec<u8> = new_inode.to_bytes();
    let new_inode_length: usize = new_inode_bytes.len();

    if new_inode_length > inode_block.bytes_free.into() {
        // We don't have room for this inode. The caller will have to use another block.
        return Err(InodeBlockError::NotEnoughSpace)
    }

    // find a spot to put our new Inode
    let offset = match find_free_space::<Inode>(&inode_block.inodes_data, new_inode_length){
        Some(ok) => ok,
        None => {
            // couldn't find enough space, block must be fragmented.
            return Err(InodeBlockError::BlockIsFragmented);
        },
    };

    // Put in the Inode
    inode_block.inodes_data[offset..offset + new_inode_length].copy_from_slice(&new_inode_bytes);

    // Subtract the new space we've taken up
    // Cast from usize to u16 should be fine in all cases,
    // how would an inode be more than 2^16 bytes? lol.
    inode_block.bytes_free -= new_inode_length as u16;

    // Return that offset, we're done.
    Ok(offset.try_into().expect("max of 503 is < u16"))
}

fn new_inode_block() -> InodeBlock {

    // Create the flags
    // By default, the bit for being the final block is set.
    let flags: InodeBlockFlags = InodeBlockFlags::FinalInodeBlockOnThisDisk;

    // An inode block with no content has 503 bytes free.
    let bytes_free: u16 = 503;

    // Since this is the final block on the disk, and we obviously cant
    // point to the next disk, since we dont know if it even exists.
    // Thus, this is the end of the Inode chain.
    let next_inode_block: u16 = u16::MAX;

    // A new inode block has no inodes in it.
    // Special care must be taken by the caller to
    // ensure to put the root inode into the root disk.
    let inodes_data: [u8; 503] = [0u8; 503];

    // all done
    InodeBlock {
        flags,
        bytes_free,
        next_inode_block,
        inodes_data,
    }
}

fn from_raw_block(block: &RawBlock) -> InodeBlock {

    // Flags
    let flags: InodeBlockFlags = InodeBlockFlags::from_bits_retain(block.data[0]);

    // Bytes free
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 into 2"));
    
    // Next inode block
    let next_inode_block: u16 = u16::from_le_bytes(block.data[3..3 + 2].try_into().expect("2 into 2"));

    // Inodes
    let inodes_data: [u8; 503] = block.data[5..5 + 503].try_into().expect("503 into 503");
    

    // All done
    InodeBlock {
        flags,
        bytes_free,
        next_inode_block,
        inodes_data,
    }
}

fn to_raw_bytes(block: &InodeBlock) -> RawBlock{
    let InodeBlock {
        flags,
        bytes_free,
        next_inode_block,
        inodes_data,
    } = block;

    let mut buffer: [u8; 512] = [0u8; 512];

    // Flags
    buffer[0] = flags.bits();

    // Bytes free
    buffer[1..1 + 2].copy_from_slice(&bytes_free.to_le_bytes());

    // next inode block
    buffer[3..3 + 2].copy_from_slice(&next_inode_block.to_le_bytes());

    // inodes
    buffer[5..5 + 503].copy_from_slice(inodes_data);

    // crc
    add_crc_to_block(&mut buffer);

    // Make the block
    let final_block: RawBlock = RawBlock {
        block_index: None,
        data: buffer,
    };

    // sanity check
    assert_eq!(block, &InodeBlock::from_bytes(&final_block));

    final_block
}

//
// impl for subtypes
//

impl Inode {
    pub(super) fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(37); // max size of an inode

        // flags
        vec.push(self.flags.bits());

        // Inode data
        // There should never be both a file and a directory in an inode.
        if self.directory.is_some() {
            vec.extend(self.directory.as_ref().unwrap().to_bytes());
        }

        if self.file.is_some() {
            vec.extend(self.file.as_ref().unwrap().to_bytes());
        }

        // Timestamps

        // Created
        vec.extend(self.created.to_bytes());
        
        // Modified
        vec.extend(self.modified.to_bytes());

        // All done.
        vec
    }

    /// Will only read the first inode in provided slice.
    /// No validation is done to check if this is a valid inode!
    /// Caller MUST ensure this is a valid slice that contains an inode starting
    /// at bit zero, otherwise no guarantees can be made about the returned inode.
    pub(super) fn from_bytes(bytes: &[u8]) -> Self {
        let mut timestamp_offset: usize = 0;

        // Flags
        let flags: InodeFlags = InodeFlags::from_bits(bytes[0]).expect("Flags should only have used bits set.");
        timestamp_offset += 1;

        // We must have the marker bit.
        assert!(flags.contains(InodeFlags::MarkerBit));

        // File or directory
        let file: Option<InodeFile> = if flags.contains(InodeFlags::FileType) {
            timestamp_offset += 12;
            Some(InodeFile::from_bytes(bytes[1..1 + 12].try_into().expect("12 = 12")))
        } else {
            None
        };
        
        let directory: Option<InodeDirectory> = if !flags.contains(InodeFlags::FileType) {
            timestamp_offset += 4;
            Some(InodeDirectory::from_bytes(bytes[1..1 + 4].try_into().expect("4 = 4")))
        } else {
            None
        };
        
        // Timestamps
        
        // Created
        let created: InodeTimestamp = InodeTimestamp::from_bytes(
            bytes[timestamp_offset..timestamp_offset + 12].try_into().expect("12 = 12")
        );
        
        // Created timestamp is 12 bytes.
        timestamp_offset += 12;
        
        // Modified
        let modified: InodeTimestamp = InodeTimestamp::from_bytes(
            bytes[timestamp_offset..timestamp_offset + 12].try_into().expect("12 = 12")
        );

        // Done.
        Self {
            flags,
            file,
            directory,
            created,
            modified,
        }
    }
}

impl InodeFile {
    fn to_bytes(&self) -> [u8; 12] {
        let mut buffer: [u8; 12] = [0u8; 12];
        buffer[..8].copy_from_slice(&self.size.to_le_bytes());
        buffer[8..].copy_from_slice(&self.pointer.to_bytes());
        buffer
    }
    fn from_bytes(bytes: [u8; 12]) -> Self {
        Self {
            size: u64::from_le_bytes(bytes[..8].try_into().expect("8 = 8")),
            pointer: DiskPointer::from_bytes(bytes[8..].try_into().expect("4 = 4")),
        }
    }
}

impl InodeDirectory {
    fn to_bytes(&self) -> [u8; 4] {
        self.pointer.to_bytes()
    }
    fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            pointer: DiskPointer::from_bytes(bytes),
        }
    }
}

impl InodeTimestamp {
    pub(super) fn to_bytes(self) -> [u8; 12] {
        let mut buffer: [u8; 12] = [0u8; 12];
        buffer[..8].copy_from_slice(&self.seconds.to_le_bytes());
        buffer[8..].copy_from_slice(&self.nanos.to_le_bytes());
        buffer
    }
    pub(super) fn from_bytes(bytes: [u8; 12]) -> Self {
        Self {
            seconds: u64::from_le_bytes(bytes[..8].try_into().expect("8 = 8")),
            nanos: u32::from_le_bytes(bytes[8..].try_into().expect("4 = 4")),
        }
    }
}

impl InodeFlags {
    pub fn new() -> Self {
        // We need the marker bit.
        InodeFlags::MarkerBit
    }
}