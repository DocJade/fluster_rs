// Information about a standard disk

// Imports
use thiserror::Error;

use crate::pool::disk::{drive_struct::HeaderConversionError, generic::block::block_structs::BlockError};

use super::block::header::header_struct::StandardDiskHeader;


// Structs, Enums, Flags

pub struct StandardDisk {
    /// Which disk is this?
    pub number: u16,
    /// The disk header
    pub header: StandardDiskHeader,
    /// The file that refers to this disk
    pub(super) disk_file: std::fs::File,
}