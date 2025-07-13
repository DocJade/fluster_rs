// Directory? Is that come kind of surgery?

use crate::disk::block::block_structs::RawBlock;
use crate::disk::block::crc::add_crc_to_block;
use crate::disk::block::directory::directory_struct::DirectoryBlock;
use crate::disk::block::directory::directory_struct::DirectoryBlockFlags;
use crate::disk::block::directory::directory_struct::DirectoryFlags;
use crate::disk::block::directory::directory_struct::DirectoryItem;
use crate::disk::block::directory::directory_struct::InodeLocation;
use crate::disk::generic_structs::pointer_struct::DiskPointer;


// Conversions back and forth for RawBlock
impl From<RawBlock> for DirectoryBlock {
    fn from(block: RawBlock) -> Self {
        Self::from_bytes(&block)
    }
}

impl From<DirectoryBlock> for RawBlock {
    fn from(block: DirectoryBlock) -> Self {
        DirectoryBlock::to_bytes(&block)
    }
}





impl DirectoryBlock {
    fn to_bytes(&self) -> RawBlock {
        directory_block_to_bytes(self)
    }
    fn from_bytes(block: &RawBlock) -> Self {
        directory_block_from_bytes(&block)
    }
}

// funtions for those impls

fn directory_block_to_bytes(block: &DirectoryBlock) -> RawBlock {
    // Deconstruct the bock
    let DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        #[allow(unused_variables)] // The items are extracted in a different way
        directory_items,
    } = block;

    let mut buffer: [u8; 512] = [0u8; 512];


    // flags
    buffer[0] = flags.bits();

    // free bytes
    buffer[1..1 + 2].copy_from_slice(&bytes_free.to_le_bytes());

    // next block on the disk
    buffer[3..3 + 4].copy_from_slice(&next_block.to_bytes());

    // Directory items
    buffer[7..7 + 501].copy_from_slice(&block.item_bytes_from_vec());

    // add the CRC
    add_crc_to_block(&mut buffer);

    // All done!
    RawBlock {
        block_index: None,
        data: buffer
    }

}

fn directory_block_from_bytes(block: &RawBlock) -> DirectoryBlock {

    // Flags
    let flags: DirectoryBlockFlags = DirectoryBlockFlags::from_bits_retain(block.data[0]);

    // Free bytes, come and get 'em
    let bytes_free: u16 = u16::from_le_bytes(block.data[1..1 + 2].try_into().expect("2 = 2"));

    // Next block
    let next_block: DiskPointer = DiskPointer::from_bytes(block.data[3..3 + 4].try_into().expect("4 = 4"));

    // The directory items
    let directory_items: Vec<DirectoryItem> = DirectoryBlock::item_vec_from_bytes(&block.data[7..7 + 501]);

    // All done
    DirectoryBlock {
        flags,
        bytes_free,
        next_block,
        directory_items,
    }
}



// Conversions for the Vec of items
impl DirectoryBlock {
    fn item_bytes_from_vec(&self) -> [u8; 501] {
        let mut index: usize = 0;
        let mut buffer: [u8; 501] = [0u8; 501];

        for i in &self.directory_items {
            for byte in i.to_bytes() {
                buffer[index] = byte;
                index += 1;
            }
        }
        buffer
    }
    fn item_vec_from_bytes(bytes: &[u8]) -> Vec<DirectoryItem> {
        let mut items: Vec<DirectoryItem> = Vec::with_capacity(83); // Theoretical limit
        let index: usize = 0;
        loop {
            // Are we out of bytes?
            if index >= bytes.len() {
                break
            }

            // Get the flags
            let flags: DirectoryFlags = DirectoryFlags::from_bits_retain(bytes[index]);

            // Check for marker bit
            if !flags.contains(DirectoryFlags::MarkerBit) {
                // No more items.
                break
            }

            // Figure out how many bytes we need to give to the converter
            let directory_size: usize = if flags.contains(DirectoryFlags::OnThisDisk) {
                // No disk number, so 3 bytes
                3
            } else {
                // Disk number is an additional 2 bytes.
                5
            };

            // Do the conversion
            let item: DirectoryItem = DirectoryItem::from_bytes(&bytes[index..index + directory_size]);

            
            // Done with this one
            items.push(item)
        }

        // All done
        items
    }
}

// Conversions for the Vec of items
impl DirectoryItem {
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(262); // Theoretical limit
        // Flags
        vec.push(self.flags.bits());

        // Item name length
        vec.push(self.name_length);

        // The name of the item
        vec.extend(self.name.as_bytes());

        // location of the inode
        vec.extend(self.location.to_bytes());

        // All done
        vec

    }
    fn from_bytes(bytes: &[u8]) -> Self {

        // Flags
        let flags: DirectoryFlags = DirectoryFlags::from_bits_retain(bytes[0]);

        // Item name length
        let name_length: u8 = bytes[1];

        // Item name
        let name: String = String::from_utf8(bytes[2..2 + name_length as usize].to_vec()).expect("File names should be valid UTF-8");

        let location: InodeLocation = InodeLocation::from_bytes(&bytes[2 + name_length as usize..]);

        Self {
            flags,
            name_length,
            name,
            location,
        }
    }
}

impl InodeLocation {
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::with_capacity(5); // Max size of this type

        // Disk number
        if self.disk.is_some() {
            vec.extend_from_slice(&self.disk.expect("Already checked").to_le_bytes());
        }

        // Block on disk
        vec.extend_from_slice(&self.block.to_le_bytes());

        // index into the block
        vec.push(self.index);

        vec
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        // Disk number
        let mut index: usize = 0;
        // we need to extract the disk number if length is 5
        let disk: Option<u16> = if bytes.len() == 5 {
            index += 2; // Offset by 2 bytes, since the next items are relative to this
            Some(u16::from_le_bytes(bytes[..2].try_into().expect("2 = 2")))
        } else {
            None
        };

        // Block on disk
        let block: u16 = u16::from_le_bytes(bytes[index..index + 2].try_into().expect("2 = 2"));

        // Index into Inode block
        // this is always the last byte
        let index: u8 = bytes[bytes.len()];
        
        Self {
            disk,
            block,
            index,
        }
    }
}