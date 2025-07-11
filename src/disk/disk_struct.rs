// Information about a disk

pub struct Disk {
    // Which disk is this?
    pub number: u16,
    // The file that refers to this disk
    pub file: std::fs::File,
}