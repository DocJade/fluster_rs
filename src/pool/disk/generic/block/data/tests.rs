// Data blocks are quite simple really.

// Imports

use rand::{RngCore, random_range, rngs::ThreadRng};

use super::data_struct::DataBlock;

// Tests

#[test]
fn random_block() {
    for _ in 0..1000 {
        let mut random: ThreadRng = rand::rng();
        let mut block: DataBlock = DataBlock::new();
        let data_size: usize = random_range(0..1024);
        let mut data: Vec<u8> = vec![0u8; data_size as usize];
        // fill it
        random.fill_bytes(&mut data);
        // write it to the block
        let amount_written: u16 = block.write(&data);
        // Read it back out
        let mut read_buffer: Vec<u8> = vec![0u8; amount_written as usize];
        let bytes_read: u16 = block.read(&mut read_buffer);

        // assertions
        assert_eq!(bytes_read, amount_written);
        assert_eq!(
            read_buffer[..bytes_read as usize],
            data[..amount_written as usize]
        );
    }
}

#[test]
fn large_store_bytes_in_blocks() {
    let mut random: ThreadRng = rand::rng();

    // Create a vector with 16-32 MB of data
    const MEGABYTE: usize = 1000000;

    // The buffer needs to be big enough to store everything.
    // Arrays cannot be used here, you will overflow the stack.
    let data_size = random_range(MEGABYTE * 16..MEGABYTE * 32);
    let mut data: Vec<u8> = vec![0u8; data_size as usize];

    // Fill-er up!
    random.fill_bytes(&mut data);

    // Write that to blocks
    // Once again, no arrays. It will overflow the stack
    let mut blocks: Vec<DataBlock> = Vec::new();
    let mut write_index: usize = 0;
    while write_index < data_size {
        let mut block = DataBlock::new();
        let bytes_written: usize = block
            .write(&data[write_index..data_size])
            .try_into()
            .unwrap();

        blocks.push(block);

        // move forwards
        write_index += bytes_written;
    }

    // read it all back off
    let mut read_buffer: Vec<u8> = vec![0u8; data_size as usize];
    let mut read_index: usize = 0;
    for block in blocks {
        let amount_read = block.read(&mut read_buffer[read_index..]);
        // Did we actually get anything?
        if amount_read == 0 {
            // Done reading
            break;
        }
        read_index += amount_read as usize;
    }

    // Check if retrieved data is the same
    assert_eq!(data, read_buffer)
}
