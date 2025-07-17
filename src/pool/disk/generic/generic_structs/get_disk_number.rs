// There are various types of disk, we need to be able to extract the disk number from any of them.

pub trait HasDiskNumber {
    /// Retrieves the disk number from this disk.
    fn get_disk_number(&self) -> u16;
}

pub fn find_free_space<T: HasDiskNumber>(disk: T) -> u16 {
    disk.get_disk_number()
}