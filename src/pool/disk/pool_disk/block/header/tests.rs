// Head in the pool? Preposterous!
// Unwrapping is okay here, since we want unexpected outcomes to fail tests.
#![allow(clippy::unwrap_used)]

// Imports
use rand::Rng;
use rand::rngs::ThreadRng;

use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;
use crate::pool::disk::pool_disk::block::header::header_struct::PoolHeaderFlags;

// Tests

// Ensure we can encode and decode a block
#[test]
fn block_ping_pong() {
    for _ in 0..1000 {
        let new_block = PoolDiskHeader::random();
        // Wizard, CAST!
        let raw_block: RawBlock = new_block.to_block();
        // Again!
        let banach_tarski: PoolDiskHeader = PoolDiskHeader::from_block(&raw_block).unwrap();

        assert_eq!(new_block, banach_tarski)
    }
}

#[cfg(test)]
impl PoolDiskHeader {
    fn random() -> Self {
        let mut random: ThreadRng = rand::rng();
        Self {
            flags: PoolHeaderFlags::random(),
            highest_known_disk: random.random(),
            disk_with_next_free_block: random.random(),
            pool_blocks_free: random.random(),
            block_usage_map: random_allocations(),
        }
    }
}

#[cfg(test)]
impl PoolHeaderFlags {
    fn random() -> Self {
        // Currently we only have the marker bit.
        PoolHeaderFlags::RequiredHeaderBit
    }
}

fn random_allocations() -> [u8; 360] {
    let mut random: ThreadRng = rand::rng();
    let mut buffer = [0u8; 360];
    for byte in buffer.iter_mut() {
        *byte = random.random()
    }
    buffer
}