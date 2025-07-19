// Da densest disk

use crate::pool::disk::dense_disk::block::header::header_struct::DenseDiskHeader;

#[derive(Debug)]
pub struct DenseDisk {
    /// The number of this disk
    pub(super) number: u16,
    /// The header for this disk
    pub(super) header: DenseDiskHeader,
    /// Map of used blocks on this disk
    pub(super) block_usage_map: [u8; 360],
    /// The disk file
    pub(super) disk_file: std::fs::File,
}
