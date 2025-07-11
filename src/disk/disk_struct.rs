// Information about a disk

use thiserror::Error;

use crate::block::{block_structs::BlockError, header::header_struct::DiskHeader};


pub struct Disk {
    // Which disk is this?
    pub number: u16, // This is just a copy of header.disk_number, i wish i could do an alias somehow
    // The disk header
    pub header: DiskHeader,
    // The file that refers to this disk
    pub(super) disk_file: std::fs::File,
}
#[derive(Debug, Error)]
pub enum DiskError {
    #[error("Disk is uninitialized")]
    Uninitialized,
    #[error("Disk is not blank")]
    NotBlank,
    #[error(transparent)]
    BlockError(#[from] BlockError)
}