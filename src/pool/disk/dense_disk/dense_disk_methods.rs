// Sometimes dense people still do things

use crate::pool::disk::drive_struct::DiskBootstrap;

use super::dense_disk_struct::DenseDisk;

impl DenseDisk {
    // todo
}

impl DiskBootstrap for DenseDisk {
    fn bootstrap(block: crate::pool::disk::generic::block::block_structs::RawBlock) -> Self {
        todo!()
    }
    
    fn from_header(block: crate::pool::disk::generic::block::block_structs::RawBlock) -> Self {
        todo!()
    }
}