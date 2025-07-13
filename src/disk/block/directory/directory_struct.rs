// Directory struct!
use bitflags::bitflags;

use crate::disk::generic_structs::pointer_struct::DiskPointer;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) struct DirectoryItem {
    pub(super) flags: DirectoryFlags,
    pub(super) name_length: u8,
    pub(super) name: String,
    pub(super) location: InodeLocation
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) struct DirectoryBlock {
    pub(super) flags: DirectoryBlockFlags,
    pub(super) bytes_free: u16,
    pub(super) next_block: DiskPointer,
    pub(super) directory_items: Vec<DirectoryItem>
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryFlags: u8 {
        const OnThisDisk = 0b00000001;
        const MarkerBit = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryBlockFlags: u8 {
    }
}

// Points to a specific inode globally
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct InodeLocation {
    pub(crate) disk: Option<u16>,
    pub(crate) block: u16,
    pub(crate) index: u8,
}