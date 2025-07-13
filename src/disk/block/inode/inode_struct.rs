// Inode layout
use bitflags::bitflags;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct Inode {
    pub(super) flags: InodeFlags,
    pub(super) file: Option<InodeFile>,
    pub(super) directory: Option<InodeDirectory>,
    pub(super) timestamp: InodeTimestamp
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodeFile {
    pub(super) size: u64,
    pub(super) pointer: InodePointer // Points to extents
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodeDirectory {
    pub(super) pointer: InodePointer // Points to directory
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Relative to Unix Epoch
pub(super) struct InodeTimestamp {
    pub(super) seconds: u64,
    pub(super) nanos: u32,
}


/// Points to a specific block on a disk
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct InodePointer {
    pub(super) disk: u16,
    pub(super) block: u16
}

/// Points to a specific inode globally
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct InodeLocation {
    pub(super) disk: Option<u16>,
    pub(super) block: u16,
    pub(super) index: u8,
}



bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeFlags: u8 {
        const FileType = 0b00000001;
        const MarkerBit = 0b10000000; // Always set
    }
}

// The block

#[derive(Debug, PartialEq, Eq)]
pub(super) struct InodeBlock {
    pub(super) flags: InodeBlockflags,
    pub(super) bytes_free: u16,
    pub(super) next_inode_block: u16,
    pub(super) inodes: Vec<Inode>
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeBlockflags: u8 {
        const FinalInodeBlockOnThisDisk = 0b00000001;
    }
}