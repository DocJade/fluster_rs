// This disk is a little stupid.

// Imports
use bitflags::bitflags;

// Structs, Enums, Flags

/// The header for dense disks
#[derive(Debug)]
pub struct DenseDiskHeader {
    /// Flags about the pool.
    pub flags: DenseDiskFlags,
    /// What disk is this?
    pub disk_number: u16,
    /// Map of used blocks on this disk
    pub block_usage_map: [u8; 360],
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DenseDiskFlags: u8 {
        // All Pool headers MUST have this bit set.
        const RequiredHeaderBit = 0b01000000;
    }
}