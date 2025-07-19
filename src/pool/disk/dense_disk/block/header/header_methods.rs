// yeah

use crate::pool::disk::{dense_disk::block::header::header_struct::DenseDiskHeader, generic::block::block_structs::RawBlock};

impl DenseDiskHeader {
    pub fn to_disk_block(&self) -> RawBlock {
        todo!();
    }
}