use crate::filesystem::item_flag::flag_struct::ItemFlag;
//
//
// ======
// Handle type
// ======
//
//


// We are in charge of our own file handle management. Fun! (lie)
// So we need a way to hand out and retrieve them.



/// Handle for any type of item (file or directory).
pub struct FileHandle {
    /// The path of this file/folder.
    pub path: Box<std::path::Path>, // Non-static size, thus boxed.
    /// Flags on this item.
    pub flags: ItemFlag
}