// Inode layout
use bitflags::bitflags;

#[derive(Debug)]
pub struct Inode {
    flags: InodeFlags,
    file: Option<InodeFile>,
    directory: Option<InodeDirectory>
}

#[derive(Debug)]
pub struct InodeFile {
    size: u64,
    pointer: InodePointer // Points to extents
}

#[derive(Debug)]
pub struct InodeDirectory {
    pointer: InodePointer // Points to directory
}

#[derive(Debug)]
/// Relative to Unix Epoch
pub struct InodeTimestamp {
    seconds: u64,
    nanos: u32,
}


/// Points to a specific block on a disk
#[derive(Debug)]
pub struct InodePointer {
    disk: u16,
    block: u16
}

/// Points to a specific inode globally
#[derive(Debug)]
pub struct InodeLocation {
    disk: Option<u16>,
    block: u16,
    index: u8,
}



bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeFlags: u8 {
        const FileType = 0b00000001;
    }
}

// The block

struct InodeBlock {
    flags: InodeBlockBitflags,
    bytes_free: u16,
    next_inode_block: u16,
    inodes: Vec<Inode>
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeBlockBitflags: u8 {
        const FinalInodeBlockOnThisDisk = 0b00000001;
    }
}