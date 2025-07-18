// Imports

use crate::pool::disk::{
    drive_struct::{FloppyDriveError, HeaderConversionError},
    generic::block::{block_structs::RawBlock, crc::add_crc_to_block},
    standard_disk::block::header::header_struct::{StandardDiskHeader, StandardHeaderFlags},
};

// Implementations

impl StandardDiskHeader {
    pub fn extract_header(raw_block: &RawBlock) -> Result<StandardDiskHeader, FloppyDriveError> {
        extract_header(raw_block)
    }
    pub fn to_disk_block(&self) -> RawBlock {
        to_disk_block(self)
    }
}

// Impl the conversion from a RawBlock to a DiskHeader
impl TryFrom<RawBlock> for StandardDiskHeader {
    type Error = FloppyDriveError;

    fn try_from(value: RawBlock) -> Result<Self, Self::Error> {
        extract_header(&value)
    }
}

// Functions

/// Extract header info from a disk
fn extract_header(raw_block: &RawBlock) -> Result<StandardDiskHeader, FloppyDriveError> {
    // Time to pull apart the header!

    // Make sure this is actually a header.
    // Check magic and block number.
    if raw_block.data[0..8] != *"Fluster!".as_bytes() || raw_block.block_index != 0 {
        // Bad input.

        // Either the disk has bad data on it, or is probably blank.
        // Check if the disk is blank
        if raw_block.data.iter().all(|&x| x == 0) {
            // The block is completely blank.
            return Err(FloppyDriveError::Uninitialized);
        }

        // Is this a fresh IBM disk?
        if raw_block.data[510..] == [0x55, 0xAA] {
            // Wow, a brand new floppy.
            return Err(FloppyDriveError::Uninitialized);
        }

        return Err(HeaderConversionError::NotAHeaderBlock.into());
    }

    // Bit flags
    let flags: StandardHeaderFlags = StandardHeaderFlags::from_bits_retain(raw_block.data[8]);

    // The disk number
    let disk_number: u16 =
        u16::from_le_bytes(raw_block.data[9..9 + 2].try_into().expect("Impossible"));

    // If this is disk zero, or the reserved pool header bit is set (bit 7),
    // that means we are currently trying to deserialize the POOL header. Abort.
    if disk_number == 0 || flags.bits() & 0b1000000 != 0 {
        return Err(HeaderConversionError::WrongHeader.into());
    }

    // block usage bitplane
    let block_usage_map: [u8; 360] = raw_block.data[148..148 + 360]
        .try_into()
        .expect("Impossible.");

    Ok(StandardDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    })
}

/// Converts the header type into its equivalent 512 byte block
fn to_disk_block(header: &StandardDiskHeader) -> RawBlock {
    // Now, this might seem stupid to reconstruct the struct immediately, but
    // doing this ensures that if the struct is updated, we have to look at this function
    // as well.

    let StandardDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    } = header;

    // Create the buffer for the header
    let mut buffer: [u8; 512] = [0u8; 512];

    // Magic numbers!
    buffer[0..8].copy_from_slice("Fluster!".as_bytes());

    // Now the flags
    buffer[8] = flags.bits();

    // The disk number
    buffer[9..9 + 2].copy_from_slice(&disk_number.to_le_bytes());

    // The block map
    buffer[148..148 + 360].copy_from_slice(block_usage_map);

    // Now CRC it
    add_crc_to_block(&mut buffer);

    // Make the RawBlock
    // Disk headers are always block 0.
    let finished_block: RawBlock = RawBlock {
        block_index: 0,
        data: buffer,
    };

    // All done!
    finished_block
}
