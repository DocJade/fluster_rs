// Imports
use bitflags::bitflags;

// Structs, Enums, Flags

/// The header of a disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct StandardDiskHeader {
    pub flags: StandardHeaderFlags,
    pub disk_number: u16,
    pub block_usage_map: [u8; 360], // not to be indexed directly, use a method to check.
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct StandardHeaderFlags: u8 {
        const Marker = 0b00100000; // Must be set.
        // 0b01000000; // Reserved for dense disk
        // 0b10000000; // Reserved for pool disk
    }
}
