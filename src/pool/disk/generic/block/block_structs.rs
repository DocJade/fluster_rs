// Structs that can be deduced from a block

// Imports
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;

// Structs, Enums, Flags

/// A raw data block
/// This should only be used internally, interfacing into this should
/// be abstracted away into other types (For example DiskHeader)
#[derive(Debug)]
pub struct RawBlock {
    /// Which block on the disk this is
    pub block_origin: DiskPointer,
    /// The block in its entirety.
    pub data: [u8; 512],
}