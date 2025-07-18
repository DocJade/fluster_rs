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
