// Tests are cool.

// Imports

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
        let test_extent = FileExtent::random();
        let serialized = test_extent.to_bytes();
        let deserialized = FileExtent::from_bytes(&serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = FileExtent::from_bytes(&re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}

#[test]
fn empty_extent_block_serialization() {
    let test_block = FileExtentBlock::new();
    let serialized = test_block.to_bytes(69);
    let deserialized = FileExtentBlock::from_bytes(&serialized);
    assert_eq!(test_block, deserialized);
}

#[test]
fn full_extent_block() {
    let mut test_block = FileExtentBlock::new();
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
        let block = FileExtentBlock::get_random();
        let serialized = block.to_bytes(69);
        let deserialized = FileExtentBlock::from_bytes(&serialized);
        assert_eq!(block, deserialized)
    }
}

// Helper functions

#[cfg(test)]
impl FileExtentBlock {
    fn get_random() -> Self {
        let mut test_block = FileExtentBlock::new();
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
        // Decide what kind of disk
        let mut flags = ExtentFlags::new();
        let disk_number: Option<u16>;
        let start_block: u16;
        let length: u8;

        if random.random_bool(0.5) {
            // Local
            flags.insert(ExtentFlags::OnThisDisk);
            disk_number = None;
            start_block = random.random();
            length = random.random();
        } else {
            // Non-local
            disk_number = Some(random.random());
            start_block = random.random();
            length = random.random();
        };

        // All done.
        FileExtent {
            flags,
            disk_number,
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
