use log::error;

use crate::pool::disk::generic::block::block_structs::RawBlock;

/// Points to a specific block on a disk

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct DiskPointer {
    pub(crate) disk: u16,
    pub(crate) block: u16,
}

impl DiskPointer {
    pub(crate) fn to_bytes(&self) -> [u8; 4] {
        let mut buffer: [u8; 4] = [0u8; 4];
        buffer[..2].copy_from_slice(&self.disk.to_le_bytes());
        buffer[2..].copy_from_slice(&self.block.to_le_bytes());
        buffer
    }
    pub(crate) fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            disk: u16::from_le_bytes(bytes[..2].try_into().expect("2 = 2")),
            block: u16::from_le_bytes(bytes[2..].try_into().expect("2 = 2")),
        }
    }
    // Random pointers for testing
    #[cfg(test)]
    pub(crate) fn get_random() -> Self {
        use rand::Rng;
        let mut random = rand::rng();
        Self {
            disk: random.random(),
            block: random.random(),
        }
    }
    /// Creates a new disk pointer with no destination.
    pub(crate) fn new_final_pointer() -> Self {
        Self {
            disk: u16::MAX,
            block: u16::MAX,
        }
    }
    /// Check if this pointer doesn't go anywhere
    pub(crate) fn no_destination(&self) -> bool {
        self.disk == u16::MAX || self.block == u16::MAX
    }
}

// Attempt to get a pointer from a raw block.
// The block must contain the disk origin.
impl From<&RawBlock> for DiskPointer {
    fn from(value: &RawBlock) -> Self {
        if value.originating_disk.is_none() {
            // We cannot get a disk pointer from a block without
            // disk information. The information MUST be present on
            // read blocks, and this call should not be made on blocks
            // that are intended to be written.
            error!("Attempted to get a disk pointer from a RawBlock that had no origin disk, we cannot continue.");
            error!("The block in question: \n{value:#?}");
            unreachable!("");
        }
        // Block is good
        Self {
            disk: value.originating_disk.expect("Guard condition."),
            block: value.block_index,
        }
    }
}

// Quick function to turn a u16 and flags into a DiskPointer
pub(in crate::pool::disk) fn u16_to_disk_pointer(
    number: u16,
    is_local: bool,
    current_disk: u16,
    default_block: u16,
) -> DiskPointer {
    if is_local {
        DiskPointer {
            disk: current_disk,
            block: number,
        }
    } else {
        DiskPointer {
            disk: number,
            block: default_block,
        }
    }
}
