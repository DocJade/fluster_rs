// Inode a block, then he moved away.

use crate::disk::block::{block_structs::RawBlock, crc::add_crc_to_block, inode::inode_struct::{Inode, InodeBlock, InodeBlockflags, InodeDirectory, InodeFile, InodeFlags, InodePointer, InodeTimestamp}};

impl From<RawBlock> for InodeBlock {
    fn from(value: RawBlock) -> Self {
        from_raw_block(&value)
    }
}

impl InodeBlock {
    pub(super) fn to_bytes(&self) -> RawBlock {
        to_raw_bytes(self)
    }
    pub(super) fn from_bytes(block: &RawBlock) -> InodeBlock {
        from_raw_block(&block)
    }
}

fn from_raw_block(block: &RawBlock) -> InodeBlock {

    // Flags
    let flags: InodeBlockflags = InodeBlockflags::from_bits_retain(block.data[0]);

    // Bytes free
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 into 2"));
    
    // Next inode block
    let next_inode_block: u16 = u16::from_le_bytes(block.data[3..3 + 2].try_into().expect("2 into 2"));

    // Inodes
    let inodes: Vec<Inode> = InodeBlock::vec_from_bytes(block.data[5..5 + 503].try_into().expect("503 into 503"));
    

    // All done
    InodeBlock {
        flags,
        bytes_free,
        next_inode_block,
        inodes,
    }
}

fn to_raw_bytes(block: &InodeBlock) -> RawBlock{
    let InodeBlock {
        flags,
        bytes_free,
        next_inode_block,
        #[allow(unused_variables)] // The inodes are extracted in a different way
        inodes,
    } = block;

    let mut buffer: [u8; 512] = [0u8; 512];

    // Flags
    buffer[0] = flags.bits();

    // Bytes free
    buffer[1..1 + 2].copy_from_slice(&bytes_free.to_le_bytes());

    // next inode block
    buffer[3..3 + 2].copy_from_slice(&next_inode_block.to_le_bytes());

    // inodes
    buffer[5..5 + 503].copy_from_slice(&block.bytes_from_vec());

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
        let timestamp: InodeTimestamp = InodeTimestamp::from_bytes(bytes[bytes.len() - 12..].try_into().expect("12 = 12"));

        Self {
            flags,
            file,
            directory,
            timestamp,
        }
    }

}

impl InodeBlock {
    pub(super) fn vec_from_bytes(bytes: &[u8]) -> Vec<Inode> {
        let mut index: usize = 0;
        let mut vec: Vec<Inode> = Vec::with_capacity(37);

        loop {
            if index >= bytes.len() {
                break
            }

            let flags = InodeFlags::from_bits_retain(bytes[index]);

            // Check for the marker bit
            if !flags.contains(InodeFlags::MarkerBit) {
                // No more inodes to read
                break
            }

            // Figure out how much we need to pass into the deserializer
            let length: usize = 4 + (8 * flags.contains(InodeFlags::FileType) as usize) + 12;
            
            // Grab the inode
            let inode = Inode::from_bytes(&bytes[index..index + length]);

            // Push it on
            vec.push(inode);

            // All done. Next!
            index += length;

        }
        // Done!
        vec
    }
    
    pub(super) fn bytes_from_vec(&self) -> [u8; 503] {
        let mut index: usize = 0;
        let mut buffer: [u8; 503] = [0u8; 503];

        for i in &self.inodes {
            for byte in i.to_bytes() {
                buffer[index] = byte;
                index += 1;
            }
        }
        buffer
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
            pointer: InodePointer::from_bytes(bytes[8..].try_into().expect("4 = 4")),
        }
    }
}

impl InodeDirectory {
    fn to_bytes(&self) -> [u8; 4] {
        self.pointer.to_bytes()
    }
    fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            pointer: InodePointer::from_bytes(bytes),
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

impl InodePointer {
    fn to_bytes(&self) -> [u8; 4] {
        let mut buffer: [u8; 4] = [0u8; 4];
        buffer[..2].copy_from_slice(&self.disk.to_le_bytes());
        buffer[2..].copy_from_slice(&self.block.to_le_bytes());
        buffer
    }
    fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            disk: u16::from_le_bytes(bytes[..2].try_into().expect("2 = 2")),
            block: u16::from_le_bytes(bytes[2..].try_into().expect("2 = 2")),
        }
    }
}