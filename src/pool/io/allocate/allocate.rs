// Pool level block allocations

use crate::pool::{disk::{drive_struct::FloppyDriveError, generic::generic_structs::pointer_struct::DiskPointer}, pool_actions::pool_struct::Pool};

impl Pool {
    /// Finds blocks across the entire pool.
    /// The blocks will be searched for only on Standard disks, all other allocations have to be done on the individual disk.
    /// If there are not enough blocks, new disks will be added as needed.
    /// Returns disk pointers for the found blocks, or a disk error.
    pub fn find_free_pool_blocks(&self, blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
        go_find_free_pool_blocks(self, blocks)
    }

    /// Creates a new dense disk and returns the disk's number
    pub fn new_dense_disk(&self) -> u16 {
        go_make_new_dense_disk(self)
    }
}


fn go_find_free_pool_blocks(pool: &Pool, blocks: u16) -> Result<Vec<DiskPointer>, FloppyDriveError> {
    todo!()
}

fn go_make_new_dense_disk(pool: &Pool) -> u16 {
    todo!()
}