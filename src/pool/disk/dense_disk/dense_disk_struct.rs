// Da densest disk

#[derive(Debug)]
pub struct DenseDisk {
    /// The number of this disk
    pub(super) number: u16,
    /// The disk file
    pub(super) disk_file: std::fs::File,
}