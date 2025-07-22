// Helper types.

use crate::pool::disk::standard_disk::block::directory::directory_struct::{DirectoryFlags, DirectoryItem};

// Need a way to search for either a file or a directory
#[derive(Ord, PartialEq, Eq, PartialOrd)]
pub enum NamedItem {
    File(String),
    Directory(String)
}
/// Specific types for named items.
impl NamedItem {
    /// Extracts the type's name, and the name of that type. (ie "file", "test.txt")
    pub fn debug_strings(&self) -> (&'static str, &String) {
        match self {
            NamedItem::File(name) => ("file", name),
            NamedItem::Directory(name) => ("directory", name),
        }
    }
    /// Search a Vec<DirectoryItem> for a NamedItem
    /// Returns the item if it exists.
    pub fn find_in(&self, to_search: &[DirectoryItem]) -> Option<DirectoryItem> {
        // Searching with this function only does the minimum amount of clones
        // to deduce if the item is present or not, instead of needing to clone the
        // entire Vec to construct the new type.
        let item_found = to_search.iter().find(|item| {
            let convert = NamedItem::from(item.clone().clone()); //TODO: This is stupid.
            convert == *self
        });
        item_found.cloned()
    }
}

/// Helper to turn DirectoryItem(s) into NamedItem(s)
impl From<DirectoryItem> for NamedItem {
    fn from(value: DirectoryItem) -> Self {
        if value.flags.contains(DirectoryFlags::IsDirectory) {
            NamedItem::Directory(value.name)
        } else {
            NamedItem::File(value.name)
        }
    }
}

