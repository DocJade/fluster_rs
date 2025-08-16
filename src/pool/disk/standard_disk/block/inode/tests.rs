// inode the tests.
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

// Imports

// Tests

use crate::error_types::block::BlockManipulationError;
use crate::pool::disk::standard_disk::block::inode::inode_struct::Inode;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeFile;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeBlock;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeFlags;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeLocation;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeDirectory;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeTimestamp;
use rand::Rng;

use test_log::test; // We want to see logs while testing.
#[test]
fn blank_inode_block_serialization() {
    // Just like the directory blocks, we must spoof the disk read.
    let block_origin = DiskPointer {
        disk: 420,
        block: 69,
    };
    let test_block: InodeBlock = InodeBlock::new(block_origin);
    let serialized = test_block.to_block();
    let deserialized = InodeBlock::from_block(&serialized);
    assert_eq!(test_block, deserialized)
}

#[test]
fn fill_inode_block() {
    let block_origin = DiskPointer {
        disk: 420,
        block: 69,
    };
    let mut test_block: InodeBlock = InodeBlock::new(block_origin);
    let mut added_inodes: Vec<Inode> = Vec::new();
    let mut inode_offsets: Vec<u16> = Vec::new();
    loop {
        let inode: Inode = Inode::get_random();
        let add_result = test_block.try_add_inode(inode);
        if add_result.is_err() {
            // It must be full, its impossible to fragment without removing items.
            assert_eq!(add_result.err().unwrap(), BlockManipulationError::OutOfRoom);
            break;
        }
        // Keep track of all added inodes so we can validate them.
        added_inodes.push(inode);
        inode_offsets.push(add_result.unwrap());
    }
    // Ensure all the inodes are present and valid.
    for i in 0..added_inodes.len() {
        // read it
        let read_inode: Inode = test_block.try_read_inode(inode_offsets[i]).unwrap();
        // Make sure they're the same.
        assert_eq!(added_inodes[i], read_inode);
    }
}

#[test]
fn filled_inode_block_serialization() {
    for _ in 0..1000 {
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let mut test_block: InodeBlock = InodeBlock::new(block_origin);
        // Fill with random inodes until we run out of room.
        loop {
            let add_result = test_block.try_add_inode(Inode::get_random());
            if add_result.is_err() {
                // It must be full, its impossible to fragment without removing items.
                assert_eq!(add_result.err().unwrap(), BlockManipulationError::OutOfRoom);
                break;
            }
        }

        // Check serialization
        let serialized = test_block.to_block();
        let deserialized = InodeBlock::from_block(&serialized);
        assert_eq!(test_block, deserialized)
    }
}

#[test]
fn add_and_read_inode() {
    for _ in 0..1000 {
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let mut test_block: InodeBlock = InodeBlock::new(block_origin);
        let inode: Inode = Inode::get_random();
        let offset = test_block.try_add_inode(inode).unwrap();
        let read_inode = test_block.try_read_inode(offset).unwrap();
        assert_eq!(inode, read_inode);
    }
}

#[test]
// Make sure the offsets are working correctly
fn inode_location_consistency() {
    for _ in 0..1000 {
        let new: InodeLocation = InodeLocation::get_random();
        let disk_number: u16 = new.pointer.disk;
        let frosted_flaked = new.to_bytes(disk_number);
        let (_, we_have_the_technology) = InodeLocation::from_bytes(&frosted_flaked, disk_number);
        assert_eq!(new, we_have_the_technology);
    }
}

#[test]
// Ensure inodes are the correct size for their subtype
fn inode_correct_sizes() {
    for _ in 0..1000 {
        let test_inode: Inode = Inode::get_random();
        if test_inode.file.is_some() {
            // A file inode should be 37 bytes long
            assert_eq!(test_inode.to_bytes().len(), 37)
        } else {
            // A directory inode should be 29 bytes long
            assert_eq!(test_inode.to_bytes().len(), 29)
        }
    }
}

#[test]
// Inodes should be the same size when re/deserializing them
fn inode_consistent_serialization() {
    for _ in 0..1000 {
        let inode: Inode = Inode::get_random();
        let serial = inode.to_bytes();
        let deserial = Inode::from_bytes(&serial);
        let re_serial = deserial.to_bytes();
        let re_deserial = Inode::from_bytes(&re_serial);

        // Original Inode survived
        assert_eq!(inode, re_deserial);

        // Intermediate did not change
        assert_eq!(deserial, re_deserial);

        // byte versions are the same
        assert_eq!(serial, re_serial);
    }
}

#[test]
fn timestamp_consistent_serialization() {
    for _ in 0..1000 {
        let inode: InodeTimestamp = InodeTimestamp::get_random();
        let serial = inode.to_bytes();
        let deserial = InodeTimestamp::from_bytes(serial);
        let re_serial = deserial.to_bytes();
        let re_deserial = InodeTimestamp::from_bytes(re_serial);

        // Original InodeTimestamp survived
        assert_eq!(inode, re_deserial);

        // Intermediate did not change
        assert_eq!(deserial, re_deserial);

        // byte versions are the same
        assert_eq!(serial, re_serial);
    }
}

// Impl to make randoms

#[cfg(test)]
impl Inode {
    pub(crate) fn get_random() -> Self {
        use rand::random_bool;
        if random_bool(0.5) {
            // A file
            let mut flags = InodeFlags::new();
            flags.insert(InodeFlags::FileType);

            Inode {
                flags,
                file: Some(InodeFile::get_random()),
                directory: None,
                created: InodeTimestamp::get_random(),
                modified: InodeTimestamp::get_random(),
            }
        } else {
            // A directory
            Inode {
                flags: InodeFlags::new(),
                file: None,
                directory: Some(InodeDirectory::get_random()),
                created: InodeTimestamp::get_random(),
                modified: InodeTimestamp::get_random(),
            }
        }
    }
}

#[cfg(test)]
impl InodeFile {
    pub(crate) fn get_random() -> Self {
        let mut random = rand::rng();
        InodeFile {
            size: random.random(),
            pointer: DiskPointer::get_random(),
        }
    }
}

#[cfg(test)]
impl InodeTimestamp {
    pub(crate) fn get_random() -> Self {
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
        let pointer: DiskPointer = DiskPointer {
            disk: random.random(),
            block: random.random(),
        };

        // random offset
        // Not testing the entire range but whatever
        let offset: u16 = random.random_range(0..250);

        InodeLocation::new(pointer, offset)
    }
}

#[cfg(test)]
impl InodeDirectory {
    pub(crate) fn get_random() -> Self {
        InodeDirectory {
            pointer: DiskPointer::get_random(),
        }
    }
}
