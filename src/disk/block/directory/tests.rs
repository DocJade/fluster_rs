// Directory tests

use rand::{self, random_bool, Rng};

#[cfg(test)]
use crate::disk::block::directory::directory_struct::DirectoryFlags;
use crate::disk::{block::{block_structs::RawBlock, directory::directory_struct::{DirectoryBlock, DirectoryBlockFlags, DirectoryItem}}, generic_structs::pointer_struct::DiskPointer};
use crate::disk::block::directory::directory_struct::InodeLocation;




// Impl for going gorilla mode, absolutely ape shit, etc

#[cfg(test)]
impl DirectoryBlock {
    fn get_random() -> DirectoryBlock {
        let mut random = rand::rng();
        // TODO: Fix this lmao, this loop never runs
        let mut random_items: Vec<DirectoryItem> = Vec::with_capacity(83); // Theoretical limit
        for i in 0..random_items.len() {
            random_items[i] = DirectoryItem::get_random();
        }
        DirectoryBlock {
            flags: DirectoryBlockFlags::get_random(),
            bytes_free: random.random(),
            next_block: DiskPointer::get_random(),
            directory_items: random_items
        }
    }
}



#[cfg(test)]
impl DirectoryFlags {
    fn get_random() -> Self {
        let mut random = rand::rng();
        DirectoryFlags::from_bits_retain(random.random())
    }
}

#[cfg(test)]
impl DirectoryBlockFlags {
    fn get_random() -> Self {
        let mut random = rand::rng();
        DirectoryBlockFlags::from_bits_retain(random.random())
    }
}

#[cfg(test)]
impl DirectoryItem {
    fn get_random() -> Self {
        use rand::distr::Alphanumeric;
        let mut random = rand::rng();
        let name_length: u8 = random.random();
        let name: String = random.sample_iter(&Alphanumeric).take(name_length.into()).map(char::from).collect();
        DirectoryItem {
            flags: DirectoryFlags::get_random(),
            name_length,
            name,
            location: InodeLocation::get_random(),
        }
    }
}