// Directory tests
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

// Imports

use rand::{self, Rng};

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryBlock;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryBlockError;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItemFlags;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItem;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeLocation;

use test_log::test; // We want to see logs while testing.

// Tests

#[test]
fn blank_directory_block_serialization() {
    // We need a origin for the block, even if nonsensical.
    let block_origin = DiskPointer {
        disk: 420,
        block: 69,
    };
    let test_block: DirectoryBlock = DirectoryBlock::new(block_origin);
    let serialized = test_block.to_block();
    let deserialized = DirectoryBlock::from_block(&serialized);
    assert_eq!(test_block, deserialized)
}

#[test]
fn directory_item_serialization() {
    for _ in 0..1000 {
        // Needs a fake disk where this block was read from.
        let fake_disk: u16 = 21; // you stupid
        let test_item = DirectoryItem::get_random();
        let serialized = test_item.to_bytes(fake_disk);
        let (_, deserialized) = DirectoryItem::from_bytes(&serialized, fake_disk);
        assert_eq!(test_item, deserialized)
    }
}

#[test]
fn filled_directory_block_serialization() {
    for _ in 0..1000 {
        // We need a origin for the block, even if nonsensical.
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let mut test_block: DirectoryBlock = DirectoryBlock::new(block_origin);
        // Fill with random inodes until we run out of room.
        loop {
            match test_block.try_add_item(&DirectoryItem::get_random()) {
                Ok(_) => continue,
                Err(err) => match err {
                    DirectoryBlockError::NotEnoughSpace => {
                        // Done filling it up
                        break;
                    }
                    _ => panic!("Got an error while adding item!"),
                },
            }
        }

        // Check serialization
        let serialized = test_block.to_block();
        let deserialized = DirectoryBlock::from_block(&serialized);
        assert_eq!(test_block, deserialized)
    }
}

#[test]
fn add_and_remove_to_directory_block() {
    for _ in 0..1000 {
        // We need a origin for the block, even if nonsensical.
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let mut test_block: DirectoryBlock = DirectoryBlock::new(block_origin);
        // Fill with random inodes until we run out of room.
        let random_item: DirectoryItem = DirectoryItem::get_random();
        test_block.try_add_item(&random_item.clone()).unwrap();
        // Make sure that went in
        assert!(!test_block.directory_items.is_empty());
        test_block.try_remove_item(&random_item).unwrap();
        // Make sure it was removed
        assert!(test_block.directory_items.is_empty());
    }
}

#[test]
fn adding_and_removing_updates_size() {
    for _ in 0..1000 {
        // We need a origin for the block, even if nonsensical.
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let mut test_block: DirectoryBlock = DirectoryBlock::new(block_origin);
        let random_item: DirectoryItem = DirectoryItem::get_random();
        let new_free = test_block.bytes_free;

        test_block.try_add_item(&random_item).unwrap();
        let added_free = test_block.bytes_free;

        test_block.try_remove_item(&random_item).unwrap();
        let removed_free = test_block.bytes_free;

        // Added should have less space
        assert!(added_free < new_free);
        // removed should have more space
        assert!(added_free < removed_free);
        // The block should be empty again
        assert!(new_free == removed_free);
    }
}

// Impl for going gorilla mode, absolutely ape shit, etc

#[cfg(test)]
impl DirectoryItemFlags {
    fn new() -> Self {
        // We always need the marker bit set
        DirectoryItemFlags::MarkerBit
    }
}

#[cfg(test)]
impl DirectoryItem {
    fn get_random() -> Self {
        let name: String = get_random_name();
        let name_length: u8 = name.len().try_into().unwrap();
        assert_eq!(name_length as usize, name.len());
        let location = InodeLocation::get_random();
        let flags = DirectoryItemFlags::new();
        DirectoryItem {
            flags,
            name_length,
            name,
            location,
        }
    }
}

#[cfg(test)]
fn get_random_name() -> String {
    // make a random string of at most 255 characters, and at least 1 character
    use rand::distr::{Alphanumeric, SampleString};
    use std::cmp::max;

    let mut random = rand::rng();
    let random_length: u8 = max(random.random(), 1); // at least one character

    // make the string
    Alphanumeric.sample_string(&mut random, random_length as usize)
}
