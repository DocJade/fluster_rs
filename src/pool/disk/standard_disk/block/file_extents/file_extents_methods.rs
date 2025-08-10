// Method acting, for extents.

// Consts
// This may change if I decide to get rid of the flags on data blocks, so here's a const.
pub(crate) const DATA_BLOCK_OVERHEAD: u64 = 5; // 1 flag, 4 checksum.

// Imports


use crate::pool::disk::{
    generic::{block::{block_structs::RawBlock, crc::add_crc_to_block}, generic_structs::pointer_struct::DiskPointer},
    standard_disk::block::file_extents::file_extents_struct::{
        ExtentFlags, FileExtent, FileExtentBlock, FileExtentBlockError, FileExtentBlockFlags,
    },
};

// Implementations

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
    pub(crate) fn from_block(block: &RawBlock) -> Self {
        from_bytes(block)
    }
    /// The destination block must be known when calling.
    pub(crate) fn to_block(&self) -> RawBlock {
        to_block(self)
    }
    /// Attempts to add a file extent to this block.
    /// 
    /// Does not write new block to disk. Caller must write it.
    ///
    /// Returns nothing
    pub(crate) fn add_extent(&mut self, extent: FileExtent) -> Result<(), FileExtentBlockError> {
        extent_block_add_extent(self, extent)
    }
    /// Create a new extent block.
    /// 
    /// Requires a destination for the block.
    ///
    /// New Extent blocks are the new final block on the disk.
    /// New Extent blocks do not point to the next block (as none exists).
    /// Caller is responsible with updating previous block to point to this new block.
    pub(crate) fn new(block_origin: DiskPointer) -> Self {
        FileExtentBlock {
            flags: FileExtentBlockFlags::default(),
            bytes_free: 501, // new blocks have 501 free bytes
            next_block: DiskPointer::new_final_pointer(),
            extents: Vec::new(),
            block_origin,
        }
    }
    /// Retrieves all extents within this _block_. NOT THE ENTIRE FILE.
    /// 
    /// If you want all of the extents that a file contains, you should be calling
    /// methods on the InodeFile itself.
    /// 
    /// Returned extents may not contain the disk component of their pointers.
    pub(crate) fn get_extents(&self) -> Vec<FileExtent> {
        // Just a layer of abstraction to prevent direct access.
        self.extents.clone()
    }
    /// Helper function that calculates how many blocks an input amount of data will require.
    /// Does not take into account the sizes of FileExtent blocks or such, just the DataBlock size.
    /// We are assuming you aren't going to write more than 32MB at a time.
    pub const fn size_to_blocks(size_in_bytes: u64) -> u16 {
        // This calculation never changes, since the overhead of block is always the same.
        // A block holds 512 bytes, but we reserve 1 bytes for the flags (Currently unused),
        // and 4 more bytes for the checksum.

        // We will always need to round up on this division.
        let mut blocks: u64;
        blocks = size_in_bytes / (512 - DATA_BLOCK_OVERHEAD);
        // If there is a remainder, we also need to add an additional block.
        if size_in_bytes % (512 - DATA_BLOCK_OVERHEAD) != 0 {
            // One more.
            blocks += 1;
        }
        // This truncates the value.
        // if you are somehow about to write a buffer of >22 floppy disks in one go, you have bigger issues.
        blocks as u16
    }
    /// Forcibly replace the extents in a FileExtentBlock.
    /// 
    /// This will also canonicalize the incoming extents. IE, if the disk in the extent matches the
    /// disk this block comes from, we will remove the disk and update flags.
    /// 
    /// You must ensure that the provided extents will fit. Otherwise this will panic.
    /// If you aren't sure that the new items will fit,
    /// you should NOT be calling this method.
    /// 
    /// This can only be called on the last extent in the chain.
    /// 
    /// Will automatically recalculate size.
    pub(in super::super::super::block) fn force_replace_extents(&mut self, new_extents: Vec<FileExtent>) {
        // Since outside callers cannot manually drain the extents from a block, this lets us make sure
        // that if you NEED to update extents, you can do that safely, and recalculate the size automatically.

        // Pull the extents in so we can modify them as needed.
        let mut to_add = new_extents;

        // Where are we?
        let our_disk = self.block_origin.disk;
        
        // Empty ourselves
        self.extents = Vec::with_capacity(to_add.len());
        
        // Yes this is a silly way to see what the default capacity of an extent block is, but im sure
        // the compiler will just optimize all of it away.
        let default_free = FileExtentBlock::new(DiskPointer::new_final_pointer()).bytes_free;

        self.bytes_free = default_free;
        
        // Now add the new extents, fixing the disk numbers as needed.
        for new in &mut to_add {
            // if the disk is the same as the block origin, we will set the local flag and such.
            if let Some(extent_disk) = new.disk_number && extent_disk == our_disk {
                // Disk matched, update the extent
                new.disk_number = None;
                new.flags.insert(ExtentFlags::OnThisDisk);
            }

            // Add it
            self.add_extent(*new).expect("Should be last extent, and new items shouldn't be too big.")
        }
        // All done.
    }
}

//
// Functions
//

fn extent_block_add_extent(
    block: &mut FileExtentBlock,
    extent: FileExtent,
) -> Result<(), FileExtentBlockError> {
    // Try and add an extent to the block

    // Since new blocks always have to go at the end of the inode chain, if there
    // is a block after this, the block needs to immediately fail.
    if !block.next_block.no_destination() {
        // Keep goin dawg, not this block.
        return Err(FileExtentBlockError::NotFinalBlock)
    }

    // figure out how big the extent is
    let extent_size: u16 = extent
        .to_bytes()
        .len()
        .try_into()
        .expect("Extents can't be > 2^16");

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
    let next_block: DiskPointer =
        DiskPointer::from_bytes(block.data[3..3 + 4].try_into().expect("4 is 4"));

    // Extract the extents in this block
    let extents: Vec<FileExtent> =
        bytes_to_extents(block.data[7..7 + 501].try_into().expect("503 bytes"));

    FileExtentBlock {
        flags,
        bytes_free,
        next_block,
        extents,
        block_origin: block.block_origin,
    }
}

fn to_block(extent_block: &FileExtentBlock) -> RawBlock {
    let FileExtentBlock {
        flags,
        next_block,
        bytes_free,
        #[allow(unused_variables)] // The extents are extracted in a different way
        extents,
        block_origin: _ } = extent_block;

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
        block_origin: extent_block.block_origin,
        data: buffer,
    };

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
            break;
        }
        // check for the marker
        let flag = ExtentFlags::from_bits_retain(bytes[offset]);
        if !flag.contains(ExtentFlags::MarkerBit) {
            // no more extents to read.
            break;
        }

        // find how many bytes long the extent is
        // yes this is silly, but idk
        let length: usize = if flag.contains(ExtentFlags::OnThisDisk) {
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

impl FileExtent {
    pub(super) fn to_bytes(self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(6);

        // flags
        vec.push(self.flags.bits());

        if !self.flags.contains(ExtentFlags::OnThisDisk) {
            // Disk number
            vec.extend_from_slice(
                &self
                    .disk_number
                    .expect("Disk numbers are present on non-local extents.")
                    .to_le_bytes(),
            );
        }

        // Start block
        vec.extend_from_slice(
            &self
                .start_block
                .to_le_bytes(),
        );
        // Length
        vec.push(
            self.length
        );
        

        vec
    }
    /// You can feed feed this too many bytes, but as long as the flag is in the right spot, it will work correctly
    pub(super) fn from_bytes(bytes: &[u8]) -> FileExtent {
        let mut offset: usize = 0;

        let flags: ExtentFlags =
            ExtentFlags::from_bits(bytes[0]).expect("Unused bits should not be set.");
        
        offset += 1;

        let disk_number: Option<u16>;
        let start_block: u16;
        let length: u8;

        // Disk number
        if flags.contains(ExtentFlags::OnThisDisk) {
            // Dont need the disk number.
            disk_number = None;
        } else {
            disk_number = Some(u16::from_le_bytes(
                bytes[offset..offset + 2].try_into().expect("2 = 2 "),
            ));
            offset += 2;
        }
        
        // Start block
        start_block = u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("2 = 2 "));
        offset += 2;

        // Length
        length = bytes[offset];

        FileExtent {
            flags,
            disk_number,
            start_block,
            length,
        }
    }

    /// Helper function that extracts all of the blocks that this extent refers to.
    /// 
    /// Only gets info about this specific extent, does no traversal.
    /// 
    /// Needs to know what disk this FileExtent came from.
    pub(crate) fn get_pointers(&self, origin_disk: u16) -> Vec<DiskPointer> {
        // Set the disk number if needed
        let disk_number: u16 = if let Some(present) = self.disk_number {
            // already there.
            present
        } else {
            // Use the passed in disk
            origin_disk
        };
        
        // Each block that the extent references
        let mut pointers: Vec<DiskPointer> = Vec::with_capacity(self.length.into());
        for n in 0..self.length {
            pointers.push(DiskPointer {
                disk: disk_number,
                block: self.start_block + n as u16
            });
        };

        pointers
    }
}

// Default bitflags
impl FileExtentBlockFlags {
    pub fn default() -> Self {
        // We aren't using any bits right now.
        FileExtentBlockFlags::empty()
    }
}
