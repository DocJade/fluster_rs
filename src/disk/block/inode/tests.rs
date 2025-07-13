// inode the tests.

#[cfg(test)]
use crate::disk::block::directory::directory_struct::InodeLocation;
use crate::disk::block::inode::inode_struct::{Inode, InodeBlock, InodeDirectory};
use crate::disk::block::inode::inode_struct::{InodeFile, InodeFlags, InodeTimestamp};
use crate::disk::block::inode::inode_struct::InodeBlockflags;
use rand::Rng;
use rand::random_bool;
use crate::disk::generic_structs::pointer_struct::DiskPointer;

#[test]
fn blank_inode_block_serialization() {
    let test_block: InodeBlock = InodeBlock::new();
    let serialized = test_block.to_bytes();
    let deserialized = InodeBlock::from_bytes(&serialized);
    assert_eq!(test_block, deserialized)
}

// Impl to make randoms

#[cfg(test)]
impl Inode {
    pub(super) fn get_random() -> Self {
        use rand::random_bool;
        let mut random = rand::rng();
        if random_bool(0.5) {
            Inode {
                flags: InodeFlags::from_bits_retain(random.random()),
                file: Some(InodeFile::get_random()),
                directory: None,
                timestamp: InodeTimestamp::get_random()
            }
        } else {
            Inode {
                flags: InodeFlags::from_bits_retain(random.random()),
                file: None,
                directory: Some(InodeDirectory::get_random()),
                timestamp: InodeTimestamp::get_random()
            }
        }
    }
}

#[cfg(test)]
impl InodeBlock {
    pub(super) fn get_random() -> Self {
        let mut random = rand::rng();
        let mut random_inodes: Vec<Inode> = Vec::with_capacity(13);
        for i in 0..random_inodes.len() {
            random_inodes[i] = Inode::get_random()
        }
        InodeBlock {
            flags: InodeBlockflags::from_bits_retain(random.random()),
            bytes_free: random.random(),
            next_inode_block: random.random(),
            inodes: random_inodes,
        }
    }
}

#[cfg(test)]
impl InodeFile {
    fn get_random() -> Self {
        let mut random = rand::rng();
        InodeFile {
            size: random.random(),
            pointer: DiskPointer::get_random(),
        }
    }
}

#[cfg(test)]
impl InodeTimestamp { 
    fn get_random() -> Self {
        let mut random = rand::rng();
        InodeTimestamp {
            seconds: random.random(),
            nanos: random.random(),
        }
    }
}

#[cfg(test)]
impl InodeLocation {
    #[cfg(test)]
    pub(crate) fn get_random() -> Self {
        let mut random = rand::rng();
        let disk: Option<u16> = if random.random_bool(0.5) {
            Some(random.random())
        } else {
            None
        };

        Self {
            disk,
            block: random.random(),
            index: random.random(),
        }
    }
}

#[cfg(test)]
impl InodeDirectory {
    fn get_random() -> Self {
        InodeDirectory {
            pointer: DiskPointer::get_random(),
        }
    }
}