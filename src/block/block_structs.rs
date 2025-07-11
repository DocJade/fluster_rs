// Structs that can be deduced from a block

use thiserror::Error;

pub struct StructuredBlock {
    // What kind of block is this?
    pub r#type: BlockType,
    // Which block is this on the disk? (0-2879 inclusive)
    pub number: u16,
    // The entire block
    pub data: [u8; 512]
}

pub enum BlockType {
    Unknown
}

// A raw data block
pub struct RawBlock {
    pub data: [u8; 512]
}

// Errors related to blocks
#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Invalid CRC checksum")]
    InvalidCRC,
}