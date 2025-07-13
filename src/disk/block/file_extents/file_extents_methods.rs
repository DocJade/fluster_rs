// Method acting, for extents.

use crate::disk::block::{block_structs::RawBlock, crc::add_crc_to_block, file_extents::file_extents_struct::{ExtentFlags, FileExtendBlockFlags, FileExtent, FileExtentBlock, FileExtentPointer}};


// Impl the conversion from RawBlock
impl From<RawBlock> for FileExtentBlock {
    fn from(value: RawBlock) -> FileExtentBlock {
        from_bytes(&value)
    }
}

// impl the extent vec to byte conversion
impl FileExtentBlock {
    pub(super) fn extents_to_bytes(&self) -> [u8; 503] {
        extents_to_bytes(&self.extents)
    }
    pub(super) fn bytes_to_extents(&mut self, bytes: [u8; 503]) {
        self.extents = bytes_to_extents(bytes)
    }
    pub(super) fn from_bytes(block: &RawBlock) -> Self {
        from_bytes(block)
    }
    pub(super) fn to_bytes(&self) -> RawBlock {
        to_bytes(self)
    }
}





fn from_bytes(block: &RawBlock) -> FileExtentBlock {

    // flags
    let flags: FileExtendBlockFlags = FileExtendBlockFlags::from_bits_retain(block.data[0]);

    // Next block
    let next_block: FileExtentPointer = FileExtentPointer::from_bytes(block.data[1..1 + 4].try_into().expect("4 is 4"));

    let extents: Vec<FileExtent> = bytes_to_extents(block.data[5..5 + 503].try_into().expect("503 bytes"));

    FileExtentBlock {
        flags,
        next_block,
        extents
    }
}

fn to_bytes(extent_block: &FileExtentBlock) -> RawBlock {

    let FileExtentBlock {
        flags,
        next_block,
        #[allow(unused_variables)] // The extents are extracted in a different way
        extents
    } = extent_block;

    let mut buffer: [u8; 512] = [0u8; 512];

    // bitflags
    buffer[0] = flags.bits();

    // Next block
    buffer[1..1 + 4].copy_from_slice(&next_block.to_bytes());

    // Extents
    buffer[5..508].copy_from_slice(&extent_block.extents_to_bytes());

    // add the CRC
    add_crc_to_block(&mut buffer);

    let finished_block: RawBlock = RawBlock {
        block_index: None,
        data: buffer
    };
    
    // make sure this matches
    assert_eq!(extent_block, &FileExtentBlock::from_bytes(&finished_block));
    
    finished_block
}

// Convert the extents to a properly sized array of bytes
fn extents_to_bytes(extents: &[FileExtent]) -> [u8; 503] {
    // I couldn't think of a nicer way to do this conversion
    let mut index: usize = 0;
    let mut buffer: [u8; 503] = [0u8; 503];

    for i in extents {
        for byte in i.to_bytes() {
            buffer[index] = byte;
            index += 1;
        }
    }
    buffer
}

// Now for the other way
fn bytes_to_extents(bytes: [u8; 503]) -> Vec<FileExtent> {
    let mut offset: usize = 0;
    let mut extent_vec: Vec<FileExtent> = Vec::new();
    // to make sure we always have at least 5 bytes to copy from any point, we need the array to be slightly bigger,
    // which needs a copy, but whatever. Maybe there's a more clever way to do this, but I dont feel bad moving 500 bytes around.

    let mut padded_bytes: [u8; 508] = [0u8; 508];
    padded_bytes[..503].copy_from_slice(&bytes);
    loop {
        // To make sure we always get 5 bytes,

        // make sure we dont go off the deep end
        if offset >= 503 {
            // cant be more.
            break
        }
        // check for the marker
        if !ExtentFlags::from_bits_retain(padded_bytes[offset]).contains(ExtentFlags::MarkerBit) {
            // no more extents to read.
            break
        }

        // read in an extent
        extent_vec.push(FileExtent::from_bytes(&bytes[offset..offset+5]));
        // increment offset
        offset += 5;
    }

    // Done!
    extent_vec
    
    
}




// Welcome to subtype impl hell

impl FileExtentPointer {
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut buffer: [u8; 4] = [0u8; 4];
        // Disk number
        buffer[..2].copy_from_slice(&self.disk_number.to_le_bytes());
        // Block on disk
        buffer[2..].copy_from_slice(&self.block_index.to_le_bytes());
        buffer
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            disk_number: u16::from_le_bytes(bytes[..2].try_into().expect("2 is 2")),
            block_index: u16::from_le_bytes(bytes[2..].try_into().expect("2 is 2")),
        }
    }

    // Helper to see if this is the last block easily
    pub fn is_final_block(&self) -> bool {
        self.block_index == u16::MAX && self.disk_number == u16::MAX
    }
}

impl FileExtent {
    pub(super) fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(6);

        // flags
        vec.push(self.flags.bits());

        if !self.flags.contains(ExtentFlags::OnThisDisk) {
            // Disk number
            vec.append(&mut self.disk_number.unwrap().to_le_bytes().to_vec());
        }
        
        if !self.flags.contains(ExtentFlags::OnDenseDisk) {
            // Start block
            vec.append(&mut self.start_block.unwrap().to_le_bytes().to_vec());
            // Length
            vec.push(self.length.unwrap());
        }

        vec

    }
    /// You can feed feed this too many bytes, but as long as the flag is in the right spot, it will work correctly
    pub(super) fn from_bytes(bytes: &[u8]) -> FileExtent {
        let flags: ExtentFlags = ExtentFlags::from_bits_retain(bytes[0]);

        // Disk number
        let disk_number: Option<u16> = if !flags.contains(ExtentFlags::OnThisDisk) {
            Some(u16::from_le_bytes(bytes[1..1 + 2].try_into().expect("2 bytes is 2 bytes")))
        } else {
            None
        };

        // Start block
        let start_block: Option<u16> = if !flags.contains(ExtentFlags::OnDenseDisk) {
            Some(u16::from_le_bytes(bytes[3..3 + 2].try_into().expect("2 bytes is 2 bytes")))
        } else {
            None
        };

        // Length
        let length: Option<u8> = if !flags.contains(ExtentFlags::OnDenseDisk) {
            Some(bytes[5])
        } else {
            None
        };

        FileExtent {
            flags,
            disk_number,
            start_block,
            length,
        }

    }
}