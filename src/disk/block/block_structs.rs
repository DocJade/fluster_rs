// Structs that can be deduced from a block

use thiserror::Error;

/// A raw data block
/// This should only be used internally, interfacing into this should
/// be abstracted away into other types (For example DiskHeader)
pub struct RawBlock {
    /// Which block on the disk this is
    pub block_index: Option<u16>,
    /// The block in its entirety.
    pub data: [u8; 512]
}

// Errors related to blocks
#[derive(Debug, Error, PartialEq)]
pub enum BlockError {
    #[error("Invalid CRC checksum")]
    InvalidCRC,
}