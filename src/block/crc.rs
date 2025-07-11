// CRC check

use crate::block::block_structs::RawBlock;

// Check whether the CRC matches the block or not
// returns true if crc matches the block correctly.
pub fn check_crc(block: &RawBlock) -> bool {
    let existing: [u8; 4] = block.data[508..512].try_into().unwrap();
    let computed: [u8; 4] = compute_crc(&block.data[0..508]);
    existing == computed
}

// Takes in the data and calculates the CRC
pub fn compute_crc(data: &[u8]) -> [u8; 4] {
    let checksum: u32 = crc32c::crc32c(data);
    checksum.to_le_bytes()
}

pub fn add_crc_to_block(block: &mut RawBlock) {
    let crc = compute_crc(&block.data[0..508]);
    block.data[508..512].copy_from_slice(&crc);
}

// TODO: Correct detected errors if possible?