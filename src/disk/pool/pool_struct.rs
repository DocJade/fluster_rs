// Struct relating to the pool of disks (IE all of the disks in use by the filesystem)

/// Somebody peed in the pool.
pub enum PoolError {
    SyncFailed,
}

#[derive(Debug)]
pub struct PoolInfo {
    pub(super) highest_known_disk: u16
}