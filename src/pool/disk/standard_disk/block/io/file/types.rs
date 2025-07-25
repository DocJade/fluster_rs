// Abstraction again? We had that yesterday!

/// A pointer to a specific byte in a data block.
pub(super) struct DataBytePointer {
    /// What disk its on
    pub disk: u16,
    /// Which block on that disk
    pub block: u16,
    /// Which byte within that block
    pub offset: u16
}