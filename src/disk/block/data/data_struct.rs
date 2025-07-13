// The data block
use bitflags::bitflags;

pub struct DataBlock {
    flags: DataBlockBitflags
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DataBlockBitflags: u8 {
    }
}