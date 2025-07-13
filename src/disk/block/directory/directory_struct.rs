// Directory struct!
use bitflags::bitflags;

use crate::disk::block::inode::inode_struct::InodeLocation;

#[derive(Debug)]
struct DirectoryItem {
    flags: DirectoryFlags,
    name_length: u8,
    name: String,
    location: InodeLocation
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DirectoryFlags: u8 {
        const OnThisDisk = 0b00000001;
    }
}