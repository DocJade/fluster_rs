use thiserror::Error;

use crate::{disk::{block::{block_structs::RawBlock, crc::{add_crc_to_block, check_crc}, header::header_struct::{DiskHeader, HeaderFlags}}, disk_struct::DiskError}, helpers::hex_view::hex_view};

impl DiskHeader {
    pub fn extract_header(raw_block: &RawBlock) -> Result<DiskHeader, DiskError> {
        extract_header(raw_block)
    }
    pub fn to_disk_block(&self) -> RawBlock {
        to_disk_block(self)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum HeaderConversionError {
    #[error("This block is not a header.")]
    NotAHeaderBlock,
}


// Impl the conversion from a RawBlock to a DiskHeader
impl TryFrom<RawBlock> for DiskHeader {
    type Error = DiskError;

    fn try_from(value: RawBlock) -> Result<Self, Self::Error> {
        extract_header(&value)
    }
}


// Functions

/// Extract header info from a disk
fn extract_header(raw_block: &RawBlock) -> Result<DiskHeader, DiskError> {
    // Time to pull apart the header!

    

    // Make sure this is actually a header.
    // Check magic and block number.
    if raw_block.data[0..8] != *"Fluster!".as_bytes() || raw_block.block_index != Some(0){
        // Bad input.

        // Either the disk has bad data on it, or is probably blank.
        // Check if the disk is blank
        if raw_block.data.iter().all(|&x| x == 0) {
            // The block is completely blank.
            return Err(DiskError::Uninitialized)
        }

        // Is this a fresh IBM disk?
        if raw_block.data[510..] == [0x55, 0xAA] {
            // Wow, a brand new floppy.
            return Err(DiskError::Uninitialized)
        }

        return Err(HeaderConversionError::NotAHeaderBlock.into())
    }

    // Bit flags
    let flags: HeaderFlags = HeaderFlags::from_bits_retain(
        raw_block.data[8]
    );

    // The disk number
    let disk_number: u16 = u16::from_le_bytes(
            raw_block.data[9..9 + 2]
            .try_into()
            .expect("Impossible")
    );
    
    // The highest disk we've seen
    let highest_known_disk: u16 = u16::from_le_bytes(
        raw_block.data[11..11 + 2]
        .try_into()
        .expect("Impossible")
    );

    // block usage bitplane
    let block_usage_map: [u8; 360] = raw_block.data[148..148 + 360]
    .try_into()
    .expect("Impossible.");

    Ok(
        DiskHeader {
            flags,
            disk_number,
            highest_known_disk,
            block_usage_map,
        }
    )

}

/// Converts the header type into its equivalent 512 byte block
fn to_disk_block(header: &DiskHeader) -> RawBlock {
    
    // Now, this might seem stupid to reconstruct the struct immediately, but
    // doing this ensures that if the struct is updated, we have to look at this function
    // as well.

    let DiskHeader {
        flags,
        disk_number,
        highest_known_disk,
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

    // The highest known disk
    buffer[11..11 + 2].copy_from_slice(&highest_known_disk.to_le_bytes());

    // The block map
    buffer[148..148 + 360].copy_from_slice(block_usage_map);

    // Now CRC it
    add_crc_to_block(&mut buffer);
    // Sanity check
    assert!(check_crc(buffer));

    // Make the RawBlock
    let finished_block: RawBlock = RawBlock {
        block_index: Some(0),
        data: buffer
    };

    // Make sure the header actually de and re-serializes properly by extracting the header again
    let reconstructed_header = extract_header(&finished_block).unwrap();

    // The header must never fail to serialize.
    assert_eq!(reconstructed_header, *header, "Header serialization issues.");

    // All done!
    finished_block
}