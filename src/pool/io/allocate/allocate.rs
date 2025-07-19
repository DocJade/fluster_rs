// Pool level block allocations

use crate::pool::{disk::generic::generic_structs::pointer_struct::DiskPointer, pool_struct::Pool};

impl Pool {
    /// Finds blocks across the entire pool.
    /// If there are not enough blocks, new disks will be added as needed.
    /// Returns disk pointers for the found blocks, or returns the number of blocks free if there is not enough space.
    pub fn find_free_pool_blocks(&self, blocks: u16) -> Result<Vec<DiskPointer>, u16> {
        go_find_free_pool_blocks(self, blocks)
    }

    /// Creates a new dense disk and returns the disk's number
    pub fn new_dense_disk(&self) -> u16 {
        go_make_new_dense_disk(self)
    }
}


fn go_find_free_pool_blocks(pool: &Pool, blocks: u16) -> Result<Vec<DiskPointer>, u16> {
    todo!()
}

fn go_make_new_dense_disk(pool: &Pool) -> u16 {
    todo!()
}