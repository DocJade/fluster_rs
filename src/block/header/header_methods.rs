use crate::block::header::header_struct::{DiskHeader, HeaderFlags};

impl DiskHeader {
    pub fn extract_header(data: [u8; 512]) -> DiskHeader {
        extract_header(data)
    }
}


// Functions

// Construct header info from a disk
fn extract_header(data: [u8; 512]) -> DiskHeader {
    // Time to pull apart the header!

    // Bit flags
    let flags: HeaderFlags = HeaderFlags::from_bits_retain(
        data[8]
    );

    // block usage bitplane
    let block_usage_map: [u8; 360] = data[149..149 + 360]
    .try_into()
    .expect("Impossible.");



    DiskHeader {
        flags,
        block_usage_map,
    }

}