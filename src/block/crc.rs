// CRC check

use crate::block::block_structs::RawBlock;

// Check wether the CRC matches the block or not
pub fn check_crc(block: RawBlock) -> bool {

}

// Takes in the first 504 of a block and calculates the CRC
pub const fn compute_crc(data: [u8; 504]) -> [u8; 4] {
    
}