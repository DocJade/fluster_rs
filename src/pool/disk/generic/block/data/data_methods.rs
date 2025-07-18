// Method acting, i mean

// Careful of stack overflows...

// Imports

use super::data_struct::DataBlock;
use std::cmp::min;

// Implementations

impl DataBlock {
    /// Write data to this block from the provided buffer.
    ///
    /// Returns number of bytes written.
    pub(super) fn write(&mut self, bytes: &[u8]) -> u16 {
        write_to_block(self, bytes)
    }
    /// Read all data from this block into the provided buffer.
    /// Buffer must be at least 506 bytes to allow a full block read.
    ///
    /// Returns number of bytes read.
    pub(super) fn read(&self, buffer: &mut [u8]) -> u16 {
        read_from_block(self, buffer)
    }
    /// Make a new empty block
    pub(super) fn new() -> Self {
        Self {
            length: 0, // empty
            data: [0u8; 508],
        }
    }
}

// Writes as many bytes as we can fit, or until we hit the end of the slice
fn write_to_block(block: &mut DataBlock, bytes: &[u8]) -> u16 {
    // Calculate how many bytes to write
    // We can write at most 508 bytes, so try to grab that many, or as much as we can.
    let number_of_bytes_to_write: u16 = min(508_usize, bytes.len())
        .try_into()
        .expect("Max is always 512");

    // Copy that many bytes in
    block.data[..number_of_bytes_to_write as usize]
        .copy_from_slice(&bytes[..number_of_bytes_to_write as usize]);

    // Update the length with how many bytes are in the block
    block.length = number_of_bytes_to_write;

    // All done.
    number_of_bytes_to_write
}

// Returns a slice of all of the written bytes on the block
fn read_from_block(block: &DataBlock, buffer: &mut [u8]) -> u16 {
    // read the data into the provided buffer
    buffer[..block.length as usize].copy_from_slice(&block.data[..block.length as usize]);
    block.length
}
