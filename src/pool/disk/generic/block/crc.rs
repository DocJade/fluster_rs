// CRC check

// Check whether the CRC matches the block or not
// returns true if crc matches the block correctly.
pub fn check_crc(block: [u8; 512]) -> bool {
    let existing: [u8; 4] = block[508..512].try_into().expect("4 = 4");
    let computed: [u8; 4] = compute_crc(&block[0..508]);
    existing == computed
}

// Takes in the data and calculates the CRC
pub fn compute_crc(bytes: &[u8]) -> [u8; 4] {
    let checksum: u32 = crc32c::crc32c(bytes);
    checksum.to_le_bytes()
}

/// Every block will always have a CRC in its last 4 bytes, regardless of block type.
pub(crate) fn add_crc_to_block(block: &mut [u8; 512]) {
    let crc = compute_crc(&block[0..508]);
    block[508..].copy_from_slice(&crc);
}

// TODO: Correct detected errors if possible?
