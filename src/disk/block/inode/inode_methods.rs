// Inode a block, then he moved away.

use crate::disk::{block::{block_structs::RawBlock, crc::add_crc_to_block, inode::inode_struct::{Inode, InodeBlock, InodeBlockError, InodeBlockFlags, InodeDirectory, InodeFile, InodeFlags, InodeReadError, InodeTimestamp}}, generic_structs::{find_space::{find_free_space, BytePingPong}, pointer_struct::DiskPointer}};

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
    /// Try to add an Inode to this block.
    /// 
    /// Returns the index of the added inode. (the first inode is 0)
    pub fn try_add_inode(&mut self, inode: Inode) -> Result<u16, InodeBlockError> {
        inode_block_try_add_inode(self, inode)
    }
    /// Create a new inode block
    /// 
    /// New Inode blocks are the new final block on the disk.
    /// New Inode blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block.
    pub fn new() -> Self {
        new_inode_block()
    }
}

//
// Functions
//

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

        // Timestamp
        vec.extend(self.timestamp.to_bytes());

        // All done.
        vec
    }

    // Will only read the first inode in provided slice.
    pub(super) fn from_bytes(bytes: &[u8]) -> Self {
        
        // Flags
        let flags: InodeFlags = InodeFlags::from_bits_retain(bytes[0]);

        // File or directory
        let file: Option<InodeFile> = if flags.contains(InodeFlags::FileType) {
            Some(InodeFile::from_bytes(bytes[1..1 + 12].try_into().expect("12 = 12")))
        } else {
            None
        };

        let directory: Option<InodeDirectory> = if !flags.contains(InodeFlags::FileType) {
            Some(InodeDirectory::from_bytes(bytes[1..1 + 4].try_into().expect("4 = 4")))
        } else {
            None
        };

        // Timestamp
        // Timestamp is offset if depending on inode type
        let timestamp_offset: usize = if flags.contains(InodeFlags::FileType) {
            12
        } else {
            4
        };

        let timestamp: InodeTimestamp = InodeTimestamp::from_bytes(
            bytes[timestamp_offset..timestamp_offset + 12].try_into().expect("12 = 12")
        );

        Self {
            flags,
            file,
            directory,
            timestamp,
        }
    }
}

impl InodeBlock {
    /// Extract a single inode from the block
    pub(super) fn extract_inode(&self, offset: u16) -> Result<Inode, InodeReadError> {
        // Bounds checking
        if offset as usize > self.inodes_data.len() {
            // We cannot read past the end of the end of the data!
            return Err(InodeReadError::ImpossibleOffset);
        }

        // get a slice with that inode and deserialize it
        return Ok(Inode::from_bytes(&self.inodes_data[offset as usize..]));
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
    fn to_bytes(&self) -> [u8; 12] {
        let mut buffer: [u8; 12] = [0u8; 12];
        buffer[..8].copy_from_slice(&self.seconds.to_le_bytes());
        buffer[8..].copy_from_slice(&self.nanos.to_le_bytes());
        buffer
    }
    fn from_bytes(bytes: [u8; 12]) -> Self {
        Self {
            seconds: u64::from_le_bytes(bytes[..8].try_into().expect("8 = 8")),
            nanos: u32::from_le_bytes(bytes[8..].try_into().expect("4 = 4")),
        }
    }
}