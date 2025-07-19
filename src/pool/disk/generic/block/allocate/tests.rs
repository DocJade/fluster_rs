// I allocate development time to testing.

use rand::{rngs::ThreadRng, Rng};

use super::block_allocation::BlockAllocation;

#[test]
/// Allocate a single block from an empty table, make sure the allocated block is in the right spot.
fn allocate_and_free_one_block() {
    let mut table = TestTable::new();

    let open_block = table.find_free_blocks(1).expect("Should have > 1 block free.");

    assert_eq!(open_block.len(), 1); // We only asked for 1 block
    assert_eq!(*open_block.first().expect("Guarded"), 0_u16); // the first block should be free
    
    let blocks_allocated = table.allocate_blocks(&open_block);
    
    assert_eq!(blocks_allocated, 1); // Should have allocated 1 block
    assert_eq!(table.block_usage_map[0], 0b10000000); // First block got set.
    
    let blocks_freed = table.free_blocks(&[0_u16].to_vec()); // free the first block
    
    assert_eq!(blocks_freed, 1); // Should have freed the block
    assert_eq!(table.block_usage_map[0], 0b00000000); // First block got freed
}

#[test]
/// Attempt to allocate more blocks than there are on a disk
/// This is a valid use-case, mass allocations like this will be used for
/// putting as much data as we can fit onto a disk.
fn oversized_allocation() {
    let table = TestTable::new();
    let open_block = table.find_free_blocks(5000).expect_err("There shouldn't be enough room.");
    assert_eq!(open_block, 2880);
}

/// Fill a table with free gaps in it
#[test]
fn saturate_table() {
    for _ in 0..1000 {
        let mut random: ThreadRng = rand::rng();
        let mut table = TestTable::new();
        // Fill with random bytes
        let mut random_table = [0u8; 360];
        for byte in random_table.iter_mut() {
            let new_byte: u8 = random.random();
            *byte = new_byte;
        }

        // Now fill up the table
        let free_blocks = table.find_free_blocks(5000).expect_err("There shouldn't be enough room.");
        let blocks_to_allocate = table.find_free_blocks(free_blocks).expect("Self reported max capacity.");
        let blocks_allocated: u16 = table.allocate_blocks(&blocks_to_allocate);

        assert_eq!(blocks_allocated, free_blocks);
        // Is it actually full tho?
        let num_unset_bits: u32 = table.block_usage_map.iter().map(|byte| byte.count_zeros()).sum();
        assert_eq!(num_unset_bits, 0);
    }
}


// We need a struct that implements the allocation methods for testing

struct TestTable {
    pub block_usage_map: [u8; 360],
}

impl TestTable {
    fn new() -> Self {
        Self {
            block_usage_map: [0u8; 360],
        }
    }
}

impl BlockAllocation for TestTable {
    fn get_allocation_table(&self) -> &[u8] {
        &self.block_usage_map
    }

    fn set_allocation_table(&mut self, new_table: &[u8]) {
        self.block_usage_map = new_table.try_into().expect("New table should be the same size as old table.");
    }
}