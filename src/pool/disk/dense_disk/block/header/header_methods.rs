// yeah

use crate::pool::disk::{
    dense_disk::block::header::header_struct::{DenseDiskFlags, DenseDiskHeader},
    generic::block::{block_structs::RawBlock, crc::add_crc_to_block},
};

impl DenseDiskHeader {
    pub fn to_disk_block(&self) -> RawBlock {
        todo!();
    }
}

/// Extract header info from a disk
fn extract_header(raw_block: &RawBlock) -> DenseDiskHeader {
    // Time to pull apart the header!

    // Bit flags
    let flags: DenseDiskFlags = DenseDiskFlags::from_bits_retain(raw_block.data[8]);

    // The disk number
    let disk_number: u16 =
        u16::from_le_bytes(raw_block.data[9..9 + 2].try_into().expect("Impossible"));

    // block usage bitplane
    let block_usage_map: [u8; 360] = raw_block.data[148..148 + 360]
        .try_into()
        .expect("Impossible.");

    DenseDiskHeader {
        flags,
        disk_number,
        block_usage_map,
    }
}

/// Converts the header type into its equivalent 512 byte block
fn to_disk_block(header: &DenseDiskHeader) -> RawBlock {
    // Now, this might seem stupid to reconstruct the struct immediately, but
    // doing this ensures that if the struct is updated, we have to look at this function
    // as well.

    let DenseDiskHeader {
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
        originating_disk: None, // This is on the way to be written.
    };

    // All done!
    finished_block
}
