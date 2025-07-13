// inode the tests.

use rand::{self, random_bool, Rng};

use crate::disk::block::inode::inode_struct::{Inode, InodeBlock, InodeBlockflags, InodeDirectory, InodeFile, InodeFlags, InodePointer, InodeTimestamp};

#[test]
fn random_inode_block_serialization() {
    for _ in 0..50000 {
        let test_block = get_random_inode_block();
        let serialized = test_block.to_bytes();
        let deserialized = InodeBlock::from(serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = InodeBlock::from(re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}

#[test]
fn random_inode_serialization() {
    for _ in 0..50000 {
        let test_inode = get_random_inode();
        let serialized = test_inode.to_bytes();
        let deserialized = Inode::from_bytes(&serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = Inode::from_bytes(&re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}

fn get_random_inode() -> Inode {
    let mut random = rand::rng();

    if random_bool(0.5) {
        Inode {
            flags: InodeFlags::from_bits_retain(random.random()),
            file: Some(get_random_inode_file()),
            directory: None,
            timestamp: get_random_inode_timestamp()
        }
    } else {
        Inode {
            flags: InodeFlags::from_bits_retain(random.random()),
            file: None,
            directory: Some(get_random_inode_directory()),
            timestamp: get_random_inode_timestamp()
        }
    }


    
}

fn get_random_inode_block() -> InodeBlock {
    let mut random = rand::rng();
    let mut random_inodes: Vec<Inode> = Vec::with_capacity(13);
    for i in 0..random_inodes.len() {
        random_inodes[i] = get_random_inode()
    }
    InodeBlock {
        flags: InodeBlockflags::from_bits_retain(random.random()),
        bytes_free: random.random(),
        next_inode_block: random.random(),
        inodes: random_inodes,
    }
}

fn get_random_inode_file() -> InodeFile {
    let mut random = rand::rng();
    InodeFile {
        size: random.random(),
        pointer: get_random_inode_pointer()
    }
}

fn get_random_inode_directory() -> InodeDirectory {
    InodeDirectory {
        pointer: get_random_inode_pointer(),
    }
}

fn get_random_inode_pointer() -> InodePointer {
    let mut random = rand::rng();
    InodePointer {
        disk: random.random(),
        block: random.random()
    }
}

fn get_random_inode_timestamp() -> InodeTimestamp {
    let mut random = rand::rng();
    InodeTimestamp {
        seconds: random.random(),
        nanos: random.random(),
    }
}