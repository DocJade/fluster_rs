// Need for type constraints

#[derive(Debug)]
pub struct BlankDisk {
    /// Every disk type needs a file!
    pub(in super::super) disk_file: std::fs::File,
}
