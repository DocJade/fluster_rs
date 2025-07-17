// Directory tests
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

use rand::{self, random_bool, Rng};

#[cfg(test)]
use crate::pool::disk::block::directory::directory_struct::DirectoryFlags;
use crate::pool::disk::{block::{block_structs::RawBlock, directory::directory_struct::{DirectoryBlock, DirectoryBlockFlags, DirectoryItem}}, generic_structs::pointer_struct::DiskPointer};
use crate::pool::disk::block::directory::directory_struct::InodeLocation;

#[test]
fn blank_directory_block_serialization() {
    let test_block: DirectoryBlock = DirectoryBlock::new();
    let serialized = test_block.to_bytes(69);
    let deserialized = DirectoryBlock::from_bytes(&serialized);
    assert_eq!(test_block, deserialized)
}

#[test]
fn directory_item_serialization() {
    for _ in 0..1000 {
        let test_item = DirectoryItem::get_random();
        let serialized = test_item.to_bytes();
        let deserialized = DirectoryItem::from_bytes(&serialized);
        assert_eq!(test_item, deserialized)
    }
}


#[test]
fn filled_directory_block_serialization() {
    for _ in 0..1000 {
        let mut test_block: DirectoryBlock = DirectoryBlock::new();
        // Fill with random inodes until we run out of room.
        loop {
            match test_block.try_add_item(DirectoryItem::get_random()) {
                Ok(_) => { break },
                Err(err) => {
                    match err {
                        crate::pool::disk::block::directory::directory_struct::DirectoryBlockError::NotEnoughSpace => todo!(),
                        _ => panic!("Got an error while adding item!")
                    }
                }
            }
        }

        // Check serialization
        let serialized = test_block.to_bytes(69);
        let deserialized = DirectoryBlock::from_bytes(&serialized);
        assert_eq!(test_block, deserialized)
    }
}

#[test]
fn add_and_remove_to_directory_block() {
    for _ in 0..1000 {
        let mut test_block: DirectoryBlock = DirectoryBlock::new();
        // Fill with random inodes until we run out of room.
        let random_item: DirectoryItem = DirectoryItem::get_random();
        test_block.try_add_item(random_item.clone()).unwrap();
        // Make sure that went in
        assert!(!test_block.directory_items.is_empty());
        test_block.try_remove_item(random_item).unwrap();
        // Make sure it was removed
        assert!(test_block.directory_items.is_empty());
    }
}

#[test]
fn adding_and_removing_updates_size() {
    for _ in 0..1000 {
        let mut test_block: DirectoryBlock = DirectoryBlock::new();
        let random_item: DirectoryItem = DirectoryItem::get_random();
        let new_free = test_block.bytes_free;
        
        test_block.try_add_item(random_item.clone()).unwrap();
        let added_free = test_block.bytes_free;
        
        test_block.try_remove_item(random_item).unwrap();
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
impl DirectoryFlags {
    fn new() -> Self {
        // We always need the marker bit set
        DirectoryFlags::MarkerBit
    }
}

#[cfg(test)]
impl DirectoryItem {
    fn get_random() -> Self {
        let name: String = get_random_name();
        let name_length: u8 = name.len().try_into().unwrap();
        assert_eq!(name_length as usize, name.len());
        let location = InodeLocation::get_random();
        let mut flags = DirectoryFlags::new();
        // Flags need to be changed if its not on this disk
        if location.disk.is_none() {
            flags.insert(DirectoryFlags::OnThisDisk);
        }
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