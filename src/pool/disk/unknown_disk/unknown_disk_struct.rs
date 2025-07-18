// Sometimes we know nothing about a disk, but we still need a type for it
// so we can satisfy type constraints on DiskType

#[derive(Debug)]
pub struct UnknownDisk {
    /// Every disk needs a file
    pub(super) disk_file: std::fs::File,
}