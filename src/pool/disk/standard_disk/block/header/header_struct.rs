// Imports
use bitflags::bitflags;

// Structs, Enums, Flags

/// The header of a disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct StandardDiskHeader {
    pub flags: HeaderFlags,
    pub disk_number: u16,
    pub block_usage_map: [u8; 360], // not to be indexed directly, use a method to check.
}


bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct StandardHeaderFlags: u8 {
        const DenseDisk = 0b00000001;
    }
}