// Information about a standard disk

// Imports

use super::block::header::header_struct::StandardDiskHeader;

// Structs, Enums, Flags
#[derive(Debug)]
pub struct StandardDisk {
    /// Which disk is this?
    pub number: u16,
    /// The disk header
    pub header: StandardDiskHeader,
    /// The file that refers to this disk
    pub(super) disk_file: std::fs::File,
}