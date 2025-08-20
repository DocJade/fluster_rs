/// Points to a specific block on a disk

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) struct DiskPointer {
    pub(crate) disk: u16,
    pub(crate) block: u16,
}

impl DiskPointer {
    #[inline]
    pub(crate) fn to_bytes(self) -> [u8; 4] {
        let mut buffer: [u8; 4] = [0u8; 4];
        buffer[..2].copy_from_slice(&self.disk.to_le_bytes());
        buffer[2..].copy_from_slice(&self.block.to_le_bytes());
        buffer
    }
    #[inline]
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
    #[inline]
    pub(crate) fn new_final_pointer() -> Self {
        Self {
            disk: u16::MAX,
            block: u16::MAX,
        }
    }
    /// Check if this pointer doesn't go anywhere
    #[inline]
    pub(crate) fn no_destination(&self) -> bool {
        self.disk == u16::MAX || self.block == u16::MAX
    }
}