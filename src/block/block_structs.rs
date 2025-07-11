// Structs that can be deduced from a block

pub struct Block {
    // What kind of block is this?
    pub r#type: BlockType,
    // Which block is this on the disk? (0-2879 inclusive)
    pub number: u16,
    // The entire block
    pub data: [u8; 512]
}

pub enum BlockType {
    Unknown
}