// Squeaky clean!

use std::{fs::File, time::Duration};

use crate::{error_types::drive::DriveError, pool::disk::generic::{
    generic_structs::pointer_struct::DiskPointer
}};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File) -> Result<(), DriveError> {
    // Bye bye!
    let chunk_size: usize = 64;
    println!("Wiping currently inserted disk...");
    let ten_blank_blocks: Vec<u8> = vec![0; 512 * chunk_size];
    
    // Write in large chunks for speed.
    for i in 0..2880/chunk_size {
        let pointer: DiskPointer = DiskPointer {
            disk: 42069_u16,
            block: (i * chunk_size) as u16,
        };
        
        // We will keep track of how long this is taking, since if a single chunk of blocks
        // takes weirdly long, chances are the disk is bad.
        let now = std::time::Instant::now();

        super::write::write_large_direct(disk, &ten_blank_blocks, pointer)?;

        if now.elapsed() > Duration::from_secs(10) {
            // Took too long, this disk is no good.
            return Err(DriveError::TakingTooLong)
        }
        
        let percent = (((i + 1) * chunk_size) as f32 / 2880_f32) * 100.0;
        println!("{percent:.1}%...");
    }
    println!("Wipe complete.");
    
    Ok(())
}