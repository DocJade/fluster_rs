// Method acting, for extents.

// Consts
// This may change if I decide to get rid of the flags on data blocks, so here's a const.
pub(crate) const DATA_BLOCK_OVERHEAD: u64 = 5; // 1 flag, 4 checksum.

// Imports


use crate::{error_types::block::BlockManipulationError, pool::disk::{
    generic::{
        block::{
            block_structs::RawBlock,
            crc::add_crc_to_block
        },
        generic_structs::pointer_struct::DiskPointer
    },
    standard_disk::block::file_extents::file_extents_struct::{
        ExtentFlags,
        FileExtent,
        FileExtentBlock,
        FileExtentBlockFlags,
    },
}};

// Implementations

// Impl the conversion from RawBlock
impl From<RawBlock> for FileExtentBlock {
    fn from(value: RawBlock) -> FileExtentBlock {
        from_bytes(&value)
    }
}

// impl the extent vec to byte conversion
impl FileExtentBlock {
    pub(super) fn extents_to_bytes(&self, destination_disk_number: u16) -> Vec<u8> {
        extents_to_bytes(&self.extents, destination_disk_number)
    }

    pub(crate) fn from_block(block: &RawBlock) -> Self {
        from_bytes(block)
    }

    /// Byte me!
    /// 
    /// This assumes you will be writing this block back to where you got it from. If this
    /// is not the case, you need to update the block origin before calling.
    pub(crate) fn to_block(&self) -> RawBlock {
        to_block(self)
    }

    /// Attempts to add a file extent to this block.
    /// 
    /// Does not write new block to disk. Caller must write it.
    ///
    /// Returns nothing
    pub(crate) fn add_extent(&mut self, extent: FileExtent) -> Result<(), BlockManipulationError> {
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
            extents: Vec::new(), // Not pre-allocated, no idea how much will end up in here.
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

    // /// Helper function that calculates how many blocks an input amount of data will require.
    // /// Does not take into account the sizes of FileExtent blocks or such, just the DataBlock size.
    // /// We are assuming you aren't going to write more than 32MB at a time.
    // pub fn size_to_blocks(size_in_bytes: u64) -> u16 {
    //     // This calculation never changes, since the overhead of block is always the same.
    //     // A block holds 512 bytes, but we reserve 1 bytes for the flags (Currently unused),
    //     // and 4 more bytes for the checksum.
    // 
    //     // We will always need to round up on this division.
    //     let mut blocks: u64;
    //     blocks = size_in_bytes / (512 - DATA_BLOCK_OVERHEAD);
    //     // If there is a remainder, we also need to add an additional block.
    //     if size_in_bytes % (512 - DATA_BLOCK_OVERHEAD) != 0 {
    //         // One more.
    //         blocks += 1;
    //     }
    //     // This truncates the value.
    //     // if you are somehow about to write a buffer of >22 floppy disks in one go, you have bigger issues.
    //     blocks as u16
    // }

    /// Forcibly replace all extents in a FileExtentBlock.
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
    pub(in super::super::super::block) fn force_replace_all_extents(&mut self, new_extents: Vec<FileExtent>) {
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
            if new.start_block.disk == our_disk {
                // Disk matched, Give the extent the local flag.
                // We don't need to update the disk number, since that'll toss itself on write.
                new.flags.insert(ExtentFlags::LocalExtent);
            } else {
                // Remove the local flag, just in case.
                new.flags.remove(ExtentFlags::LocalExtent);
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

/// Add an extent to a block. Returns false if extent could not fit.
fn extent_block_add_extent(
    block: &mut FileExtentBlock,
    extent: FileExtent,
) -> Result<(), BlockManipulationError> {
    // Try and add an extent to the block

    // Yes this causes a lot of extent.to_bytes() calls, but
    // we need to be able to toss the disk number.
    // In theory this could be kept track of at a higher level than this.
    // TODO: Maybe someday.

    // What block this is going into
    let destination_disk_number: u16 = block.block_origin.disk;

    // Since new blocks always have to go at the end of the inode chain, if there
    // is a block after this, the block needs to immediately fail.
    if !block.next_block.no_destination() {
        // Keep goin dawg, not this block.
        return Err(BlockManipulationError::NotFinalBlockInChain)
    }

    // figure out how big the extent is.
    // This always less than 2^16 bytes, truncation is fine.
    let extent_size: u16 = extent.to_bytes(destination_disk_number).len() as u16;

    // will it fit?
    if extent_size > block.bytes_free {
        // Nope!
        return Err(BlockManipulationError::OutOfRoom);
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
    
    // What block this came from
    let origin_disk = block.block_origin.disk;

    // bytes free
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 = 2"));

    // Next block
    let next_block: DiskPointer =
        DiskPointer::from_bytes(block.data[3..3 + 4].try_into().expect("4 = 4"));

    // Extract the extents in this block
    let extent_data = &block.data[7..7 + 501];
    let extents: Vec<FileExtent> = bytes_to_extents(extent_data, origin_disk);

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
        block_origin: origin // We assume the block will be written back to the same spot it came from.
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
    buffer[index..index + 501].copy_from_slice(&extent_block.extents_to_bytes(origin.disk));

    // add the CRC
    add_crc_to_block(&mut buffer);

    let finished_block: RawBlock = RawBlock {
        block_origin: extent_block.block_origin,
        data: buffer,
    };

    finished_block
}

// Convert the extents to a properly sized array of bytes
fn extents_to_bytes(extents: &[FileExtent], destination_disk_number: u16) -> Vec<u8> {
    // I couldn't think of a nicer way to do this conversion
    let mut index: usize = 0;
    let mut buffer: [u8; 501] = [0u8; 501];

    for i in extents {
        for byte in i.to_bytes(destination_disk_number) {
            buffer[index] = byte;
            index += 1;
        }
    }
    buffer.to_vec()
}

// Takes in bytes and makes extents, automatically determines when to stop.
fn bytes_to_extents(bytes: &[u8], origin_disk_number: u16) -> Vec<FileExtent> {
    let mut offset: usize = 0;

    // As stated in `to_bytes` file extents are at most 6 bytes, so we will pre-allocate
    // room for a totally full extent block, which right now is 501 bytes.
    let mut extent_vec: Vec<FileExtent> = Vec::with_capacity(501_usize.div_ceil(6));

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

        // read in an extent
        let (bytes_used, new_extent) = FileExtent::from_bytes(&bytes[offset..], origin_disk_number);
        extent_vec.push(new_extent);
        // increment offset
        offset += bytes_used as usize;
    }

    // Done!
    extent_vec
}

// Welcome to subtype impl hell

impl FileExtent {
    /// Must provide what disk the FileExtentBlock that contains this FileExtent will end up on.
    /// 
    /// Ignores incoming Local flag, will update flags automatically.
    pub(super) fn to_bytes(mut self, destination_disk_number: u16) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(6); // At most 6 bytes.

        // If the disk number is the same, we set the local flag.
        if self.start_block.disk == destination_disk_number {
            self.flags.insert(ExtentFlags::LocalExtent);
        } else {
            // Otherwise we wont. duh
            self.flags.remove(ExtentFlags::LocalExtent);
        }

        // flags
        vec.push(self.flags.bits());

        if !self.flags.contains(ExtentFlags::LocalExtent) {
            // Disk number
            vec.extend_from_slice(
                &self
                    .start_block.disk
                    .to_le_bytes(),
            );
        }

        // Start block
        vec.extend_from_slice(
            &self
                .start_block
                .block
                .to_le_bytes(),
        );
        // Length
        vec.push(
            self.length
        );
        

        vec
    }
    /// You can feed feed this too many bytes, but as long as the flag is in the right spot, it will work correctly.
    /// 
    /// Also returns how many bytes the read extent was made of.
    pub(super) fn from_bytes(bytes: &[u8], origin_disk_number: u16) -> (u8, FileExtent) {
        let mut offset: usize = 0; // Extents are always <=6 bytes, so we cast this later

        let flags: ExtentFlags =
            ExtentFlags::from_bits(bytes[0]).expect("Unused bits should not be set.");
        
        offset += 1;

        let disk_number: u16;

        // Disk number
        if flags.contains(ExtentFlags::LocalExtent) {
            // Use the provided disk number.
            disk_number = origin_disk_number;
        } else {
            disk_number = u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("2 = 2 "),);
            offset += 2;
        }
        
        // Start block
        let start_block: u16 = u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("2 = 2 "));
        offset += 2;

        // Length
        let length: u8 = bytes[offset];

        // Final offset increment, since we are also using this to track size.
        offset += 1;

        // Construct a pointer for the start block
        let start_block: DiskPointer = DiskPointer {
            disk: disk_number,
            block: start_block,
        };

        // Return the number of bytes this was constructed from, and the extent
        let the_extent_of_it = FileExtent {
            flags,
            start_block,
            length,
        };

        (offset as u8, the_extent_of_it)
    }

    /// Helper function that extracts all of the blocks that this extent refers to.
    /// 
    /// Only gets info about this specific extent, does no traversal.
    /// 
    /// Needs to know what disk this FileExtent came from.
    pub(crate) fn get_pointers(&self) -> Vec<DiskPointer> {
        // Each block that the extent references
        let mut pointers: Vec<DiskPointer> = Vec::with_capacity(self.length.into());
        for n in 0..self.length {
            pointers.push(DiskPointer {
                disk: self.start_block.disk,
                block: self.start_block.block + n as u16
            });
        };
        pointers
    }

    /// Make a new file extent
    pub(crate) fn new(start_block: DiskPointer, length: u8) -> Self {
        Self {
            // These flags will be calculated on write.
            flags: ExtentFlags::MarkerBit,
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
