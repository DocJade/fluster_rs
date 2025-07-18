// Method acting, for extents.

// Imports


// Implementations

use crate::pool::disk::{generic::block::{block_structs::RawBlock, crc::add_crc_to_block}, standard_disk::block::file_extents::file_extents_struct::{ExtentFlags, FileExtent, FileExtentBlock, FileExtentBlockError, FileExtentBlockFlags, FileExtentPointer}};

// Impl the conversion from RawBlock
impl From<RawBlock> for FileExtentBlock {
    fn from(value: RawBlock) -> FileExtentBlock {
        from_bytes(&value)
    }
}

// impl the extent vec to byte conversion
impl FileExtentBlock {
    pub(super) fn extents_to_bytes(&self) -> Vec<u8> {
        extents_to_bytes(&self.extents)
    }
    pub(super) fn bytes_to_extents(&mut self, bytes: &[u8]) {
        self.extents = bytes_to_extents(bytes)
    }
    pub(super) fn from_bytes(block: &RawBlock) -> Self {
        from_bytes(block)
    }
    /// The destination block must be known when calling.
    pub(super) fn to_bytes(&self, block_number: u16) -> RawBlock {
        to_bytes(self, block_number)
    }
    /// Attempts to add a file extent to this block
    /// 
    /// Returns nothing
    pub(super) fn add_extent(&mut self, extent: FileExtent) -> Result<(), FileExtentBlockError> {
        extent_block_add_extent(self, extent)
    }
    /// Create a new extent block.
    /// 
    /// New Extent blocks are the new final block on the disk.
    /// New Extent blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block.
    pub(super) fn new() -> Self {
        FileExtentBlock {
            flags: FileExtentBlockFlags::default(),
            bytes_free: 501, // new blocks have 501 free bytes
            next_block: FileExtentPointer::final_block(),
            extents: Vec::new(),
        }
    }
    /// Reterieves all extents within this block.
    pub(super) fn get_extents(&self) -> Vec<FileExtent> {
        // Just a layer of abstraction to prevent direct access.
        self.extents.clone()
    }
}



//
// Functions
//

fn extent_block_add_extent(block: &mut FileExtentBlock, extent: FileExtent) -> Result<(), FileExtentBlockError> {
    // Try and add an extent to the block

    // figure out how big the extent is
    let extent_size: u16 = extent.to_bytes().len().try_into().expect("Extents can't be > 2^16");

    // will it fit?
    if extent_size > block.bytes_free {
        // Nope!
        return Err(FileExtentBlockError::NotEnoughSpace);
    }

    // It'll fit! Add it to the Vec.
    block.extents.push(extent);

    // we are using that space now.
    block.bytes_free -= extent_size;

    Ok(())
}

fn from_bytes(block: &RawBlock) -> FileExtentBlock {

    // flags
    let flags: FileExtentBlockFlags = FileExtentBlockFlags::from_bits_retain(block.data[0]);

    // bytes free
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 = 2"));

    // Next block
    let next_block: FileExtentPointer = FileExtentPointer::from_bytes(block.data[3..3 + 4].try_into().expect("4 is 4"));

    let extents: Vec<FileExtent> = bytes_to_extents(block.data[7..7 + 501].try_into().expect("503 bytes"));

    FileExtentBlock {
        flags,
        bytes_free,
        next_block,
        extents
    }
}

fn to_bytes(extent_block: &FileExtentBlock, block_number: u16) -> RawBlock {

    let FileExtentBlock {
        flags,
        next_block,
        bytes_free,
        #[allow(unused_variables)] // The extents are extracted in a different way
        extents
    } = extent_block;

    let mut buffer: [u8; 512] = [0u8; 512];
    let mut index: usize = 0;

    // bitflags
    buffer[index] = flags.bits();
    index += 1;
    
    // free bytes
    buffer[index..index + 2].copy_from_slice(&bytes_free.to_le_bytes());
    index += 2;
    
    // Next block
    buffer[index..index + 4].copy_from_slice(&next_block.to_bytes());
    index += 4;

    // Extents
    buffer[index..index + 501].copy_from_slice(&extent_block.extents_to_bytes());

    // add the CRC
    add_crc_to_block(&mut buffer);

    let finished_block: RawBlock = RawBlock {
        block_index: block_number,
        data: buffer
    };
    
    // make sure this matches
    assert_eq!(extent_block, &FileExtentBlock::from_bytes(&finished_block));
    
    finished_block
}

// Convert the extents to a properly sized array of bytes
fn extents_to_bytes(extents: &[FileExtent]) -> Vec<u8> {
    // I couldn't think of a nicer way to do this conversion
    let mut index: usize = 0;
    let mut buffer: [u8; 501] = [0u8; 501];

    for i in extents {
        for byte in i.to_bytes() {
            buffer[index] = byte;
            index += 1;
        }
    }
    buffer.to_vec()
}

// Now for the other way
fn bytes_to_extents(bytes: &[u8]) -> Vec<FileExtent> {
    let mut offset: usize = 0;
    let mut extent_vec: Vec<FileExtent> = Vec::new();

    loop {
        // make sure we dont go off the deep end
        if offset >= bytes.len() {
            // cant be more.
            break
        }
        // check for the marker
        let flag = ExtentFlags::from_bits_retain(bytes[offset]);
        if !flag.contains(ExtentFlags::MarkerBit) {
            // no more extents to read.
            break
        }

        // find how many bytes long the extent is
        // yes this is silly, but idk
        let length: usize = if flag.contains(ExtentFlags::OnDenseDisk) {
            3
        } else if flag.contains(ExtentFlags::OnThisDisk) {
            4
        } else {
            6
        };


        // read in an extent
        let new_extent = FileExtent::from_bytes(&bytes[offset..offset + length]);
        extent_vec.push(new_extent);
        // increment offset
        offset += new_extent.to_bytes().len();
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
    pub(super) fn to_bytes(self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(6);

        // flags
        vec.push(self.flags.bits());

        if !self.flags.contains(ExtentFlags::OnThisDisk) {
            // Disk number
            vec.extend_from_slice(&self.disk_number.expect("Disk numbers are present on non-local extents.").to_le_bytes());
        }
        
        if !self.flags.contains(ExtentFlags::OnDenseDisk) {
            // Start block
            vec.extend_from_slice(&self.start_block.expect("Start blocks are on all non-dense file extents.").to_le_bytes());
            // Length
            vec.push(self.length.expect("If we have a start block, we should also have a length."));
        }

        vec

    }
    /// You can feed feed this too many bytes, but as long as the flag is in the right spot, it will work correctly
    pub(super) fn from_bytes(bytes: &[u8]) -> FileExtent {
        let flags: ExtentFlags = ExtentFlags::from_bits(bytes[0]).expect("Unused bits should not be set.");
        println!("{}",bytes.len());
        // 3 distinct disk types as of writing.
        // cleaner implementation is probably possible, but for just 3 types? this is fine

        let disk_number: Option<u16>;
        let start_block: Option<u16>;
        let length: Option<u8>;

        // Dense disk
        if flags.contains(ExtentFlags::OnDenseDisk) {
            disk_number = Some(u16::from_le_bytes(bytes[1..1 + 2].try_into().expect("2 = 2 ")));
            start_block = None;
            length = None;
        } else if flags.contains(ExtentFlags::OnThisDisk) {
            // Local
            disk_number = None;
            start_block = Some(u16::from_le_bytes(bytes[1..1 + 2].try_into().expect("2 = 2 ")));
            length = Some(bytes[3]);
        } else {
            // Neither.
            disk_number = Some(u16::from_le_bytes(bytes[1..1 + 2].try_into().expect("2 = 2 ")));
            start_block = Some(u16::from_le_bytes(bytes[3..3 + 2].try_into().expect("2 = 2 ")));
            length = Some(bytes[5]);
        }

        FileExtent {
            flags,
            disk_number,
            start_block,
            length,
        }

    }
}


// Default bitflags
impl FileExtentBlockFlags {
    pub fn default() -> Self {
        // We aren't using any bits right now.
        FileExtentBlockFlags::empty()
    }
}

// Final block
impl FileExtentPointer {
    const fn final_block() -> Self {
        FileExtentPointer {
            disk_number: u16::MAX,
            block_index: u16::MAX,
        }
    }
}