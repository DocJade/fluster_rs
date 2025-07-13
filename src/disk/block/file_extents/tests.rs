// Tests are cool.

use crate::disk::block::file_extents::file_extents_struct::{ExtentFlags, FileExtent};
use rand::{self, Rng};

#[test]
fn random_serial_extents() {
    // Make some random extents and de/serialize them
    let mut random = rand::rng();
    for _ in 0..50000 {
        let test_extent = FileExtent {
            flags: ExtentFlags::from_bits_retain(random.random::<u8>() & ExtentFlags::MarkerBit.bits()),
            disk_number: Some(random.random()),
            start_block: Some(random.random()),
            length: Some(random.random()),
        };
        let serialized = test_extent.to_bytes();
        let deserialized = FileExtent::from_bytes(&serialized);
        let re_serialized = deserialized.to_bytes();
        let re_deserialized = FileExtent::from_bytes(&re_serialized);
        assert_eq!(deserialized, re_deserialized)
    }
}