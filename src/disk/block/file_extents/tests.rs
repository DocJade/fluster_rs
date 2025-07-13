// Tests are cool.

use crate::disk::block::file_extents::file_extents_struct::{ExtentFlags, FileExtendBlockFlags, FileExtent, FileExtentBlock, FileExtentPointer};
use rand::{self, Rng};

#[test]
fn random_extents_serialization() {
    // Make some random extents and de/serialize them
    for _ in 0..50000 {
        let test_extent = random_extent();
        let serialized = test_extent.to_bytes();
        let deserialized = FileExtent::from_bytes(&serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = FileExtent::from_bytes(&re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}

#[test]
fn random_block_serialization() {
    for _ in 0..50000 {
        let test_block = random_file_extent_block();
        let serialized = test_block.to_bytes();
        let deserialized = FileExtentBlock::from_bytes(&serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = FileExtentBlock::from_bytes(&re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}




// Helper functions


fn random_extent() -> FileExtent {
    let mut random = rand::rng();
    FileExtent {
        flags: ExtentFlags::from_bits_retain(random.random::<u8>() & ExtentFlags::MarkerBit.bits()),
        disk_number: Some(random.random()),
        start_block: Some(random.random()),
        length: Some(random.random()),
    }
}

fn random_pointer() -> FileExtentPointer {
    let mut random = rand::rng();
    FileExtentPointer {
        disk_number: random.random(),
        block_index: random.random()
    }
}

fn random_file_extent_block() -> FileExtentBlock {
    let mut random = rand::rng();
    let mut random_extents: Vec<FileExtent> = Vec::with_capacity(100);
    for i in 0..random_extents.len() {
        random_extents[i] = random_extent()
    }

    FileExtentBlock {
        flags: FileExtendBlockFlags::from_bits_retain(random.random()),
        next_block: random_pointer(),
        extents: random_extents
    }
}