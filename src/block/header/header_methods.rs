use crate::{block::{block_structs::BlockError, crc::{add_crc_to_block, check_crc, compute_crc}, header::header_struct::{DiskHeader, HeaderFlags}}, helpers::hex_view::hex_view};

impl DiskHeader {
    pub fn extract_header(data: [u8; 512]) -> Result<DiskHeader, BlockError> {
        extract_header(data)
    }
    pub fn to_disk_block(&self) -> [u8; 512] {
        to_disk_block(self)
    }
}


// Functions

/// Extract header info from a disk
fn extract_header(data: [u8; 512]) -> Result<DiskHeader, BlockError> {
    // Time to pull apart the header!

    // Make sure this is actually a header.
    if data[0..8] != *"Fluster!".as_bytes() {
        // Bad input.
        hex_view(data.to_vec());
        panic!("A non-header file was fed into extract_header!")
    }

    // Check the CRC!
    if !check_crc(data) {
        // Bad CRC!
        // TODO: Let extract_header's caller use the usual block reading
        // TODO: calls, then perform the crc after reading every block.
        return Err(BlockError::InvalidCRC);
    }

    // Bit flags
    let flags: HeaderFlags = HeaderFlags::from_bits_retain(
        data[8]
    );

    // The disk number
    let disk_number: u16 = u16::from_le_bytes(
            data[9..9 + 2]
            .try_into()
            .expect("Impossible")
        );
    

    // block usage bitplane
    let block_usage_map: [u8; 360] = data[148..148 + 360]
    .try_into()
    .expect("Impossible.");

    Ok(
        DiskHeader {
            flags,
            disk_number,
            block_usage_map,
        }
    )

}

/// Converts the header type into its equivalent 512 byte block
fn to_disk_block(header: &DiskHeader) -> [u8; 512] {
    
    // Now, this might seem stupid to reconstruct the struct immediately, but
    // doing this ensures that if the struct is updated, we have to look at this function
    // as well.

    let DiskHeader {
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
    buffer[9..9 + 2].copy_from_slice(&disk_number.to_be_bytes());

    // The block map
    buffer[148..148 + 360].copy_from_slice(block_usage_map);

    // Now CRC it
    add_crc_to_block(&mut buffer);
    // Sanity check
    assert!(check_crc(buffer));

    // Make sure the header actually de and re-serializes properly by extracting the header again
    let reconstructed_header = extract_header(buffer).expect("Original header should be valid.");

    
    if reconstructed_header != *header {
        // The header does not match. Bad news.
        println!("=========");
        println!("Reconstruction:\n{reconstructed_header:#?}");
        println!("+++++++++");
        println!("Goal:\n{header:#?}");
        println!("=========");
        panic!("Header serialization is malformed!")
    };

    // All done!
    buffer
}