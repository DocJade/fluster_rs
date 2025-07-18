// The data block

/// You must pass the data to put into the block on creation
#[derive(Debug, Clone, Copy)]
pub struct DataBlock {
    // Number of data bytes on the disk
    // You can only interact with this data via methods.
    pub(super) length: u16,
    pub(super) data: [u8; 508], // The last
}
