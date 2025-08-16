// Directory struct!

// Imports

use bitflags::bitflags;
use thiserror::Error;

use crate::pool::disk::{
    generic::generic_structs::pointer_struct::DiskPointer,
    standard_disk::block::inode::inode_struct::InodeLocation,
};

// Structs / Enums / Flags

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DirectoryItem {
    pub flags: DirectoryItemFlags,
    pub name_length: u8,
    pub name: String,
    pub location: InodeLocation,
}

// This type is not clone, since you could end up with a block that is out of sync due to
// changes made on a copy/clone of it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DirectoryBlock {
    pub(super) flags: DirectoryBlockFlags,
    pub(super) bytes_free: u16,
    // Points to the next directory block.
    // Directories are separate from each other, you cannot get from one directory to another by just following
    // the next block pointer. This pointer represents a _continuation_ of the current directory.
    pub next_block: DiskPointer,
    // At runtime its useful to know where this block came from.
    // This doesn't need to get written to disk.
    pub block_origin: DiskPointer, // This MUST be set. it cannot point nowhere.
    pub(crate) directory_items: Vec<DirectoryItem>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryItemFlags: u8 {
        const IsDirectory = 0b00000010; // Set if directory
        const MarkerBit = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryBlockFlags: u8 {
        // Currently unused.
    }
}