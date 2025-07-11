use bitflags::bitflags;


/// The header of a disk
pub struct DiskHeader {
    pub flags: HeaderFlags,
    pub block_usage_map: [u8; 360], // not to be indexed directly, use a method to check.
}

bitflags! {
    pub struct HeaderFlags: u8 {
        const DenseDisk = 0b00000001;
    }
}