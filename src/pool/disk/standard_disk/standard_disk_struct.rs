// Information about a standard disk

// Imports
use thiserror::Error;


// Structs, Enums, Flags

pub struct Disk {
    // Which disk is this?
    pub number: u16, // This is just a copy of header.disk_number, i wish i could do an alias somehow
    // The disk header
    pub header: DiskHeader,
    // The file that refers to this disk
    pub(super) disk_file: std::fs::File,
}
#[derive(Debug, Error, PartialEq)]
pub enum DiskError {
    #[error("Disk is uninitialized")]
    Uninitialized,
    #[error("Disk is not blank")]
    NotBlank,
    #[error("Wipe failed, disk is in an unknown state.")]
    WipeFailure,
    // we'll put this back in later if we need it.
    // #[error("There isn't a disk inserted")]
    // NoDiskInserted,
    #[error("This is not the disk we want")]
    WrongDisk,
    #[error(transparent)]
    BadHeader(#[from] HeaderConversionError),
    #[error(transparent)]
    BlockError(#[from] BlockError),
}