// Tests are cool.

// Imports

use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

use super::file_extents_struct::ExtentFlags;
use super::file_extents_struct::FileExtent;
use super::file_extents_struct::FileExtentBlock;
use rand::Rng;
use rand::rngs::ThreadRng;

use test_log::test; // We want to see logs while testing.

// Tests

#[test]
fn random_extents_serialization() {
    // Make some random extents and de/serialize them
    for _ in 0..1000 {
        // Need a test disk number for the serialization to happen on
        let origin_disk: u16 = 42;
        let test_extent = FileExtent::random();
        let serialized = test_extent.to_bytes(origin_disk);
        let (de_len, deserialized) = FileExtent::from_bytes(&serialized, origin_disk);
        let re_serialized = deserialized.to_bytes(origin_disk);
        let (re_de_len, re_deserialized) = FileExtent::from_bytes(&re_serialized, origin_disk);
        assert_eq!(deserialized, re_deserialized);
        assert_eq!(de_len, re_de_len);
    }
}

#[test]
fn empty_extent_block_serialization() {
    let block_origin = DiskPointer {
        disk: 420,
        block: 69,
    };
    let test_block = FileExtentBlock::new(block_origin);
    let serialized = test_block.to_block();
    let deserialized = FileExtentBlock::from_block(&serialized);
    assert_eq!(test_block, deserialized);
}

#[test]
fn full_extent_block() {
    let block_origin = DiskPointer {
        disk: 420,
        block: 69,
    };
    let mut test_block = FileExtentBlock::new(block_origin);
    let mut extents: Vec<FileExtent> = Vec::new();
    loop {
        let new_extent: FileExtent = FileExtent::random();
        match test_block.add_extent(new_extent) {
            Ok(_) => {
                // keep track of the extents we put in
                extents.push(new_extent);
                // keep going
            }
            Err(err) => match err {
                super::file_extents_struct::FileExtentBlockError::NotEnoughSpace => break, // full
                _ => panic!("This only happens on one block, how is this not the final block?")
            },
        }
    }
    // Make sure all of the extents stored correctly
    let retrieved_extents: Vec<FileExtent> = test_block.get_extents();
    assert!(extents.iter().all(|item| retrieved_extents.contains(item)));
}

#[test]
fn random_block_serialization() {
    for _ in 0..1000 {
        // We need a origin for the block, even if nonsensical.
        let block_origin = DiskPointer {
            disk: 420,
            block: 69,
        };
        let test_block = FileExtentBlock::get_random(block_origin);
        let serialized = test_block.to_block();
        let deserialized = FileExtentBlock::from_block(&serialized);
        assert_eq!(test_block, deserialized)
    }
}

// Helper functions

#[cfg(test)]
impl FileExtentBlock {
    fn get_random(block_origin: DiskPointer) -> Self {
        let mut test_block = FileExtentBlock::new(block_origin);
        let mut random: ThreadRng = rand::rng();
        // Fill with a random amount of items.
        loop {
            // consider stopping early
            if random.random_bool(0.50) {
                break;
            }
            let new_extent: FileExtent = FileExtent::random();
            match test_block.add_extent(new_extent) {
                Ok(_) => {}
                Err(err) => match err {
                    super::file_extents_struct::FileExtentBlockError::NotEnoughSpace => break, // full
                    _ => panic!("This only happens on one block, how is this not the final block?")
                },
            }
        }
        test_block
    }
}

#[cfg(test)]
impl FileExtent {
    fn random() -> Self {
        let mut random: ThreadRng = rand::rng();
        // Flags do not matter, they are auto deduced.
        let flags = ExtentFlags::new();
        let length: u8 = random.random();
        let start_block: DiskPointer = DiskPointer::get_random();

        // All done.
        FileExtent {
            flags,
            start_block,
            length,
        }
    }
}

#[cfg(test)]
impl ExtentFlags {
    fn new() -> Self {
        // always need the marker bit.
        let mut flag = ExtentFlags::empty();
        flag.insert(ExtentFlags::MarkerBit);
        flag
    }
}
