// Squeaky clean!

use std::fs::File;

use crate::{error_types::drive::DriveError, pool::disk::generic::{
    generic_structs::pointer_struct::DiskPointer
}};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File) -> Result<(), DriveError> {
    // Bye bye!
    println!("Wiping currently inserted disk...");
    let ten_blank_blocks: Vec<u8> = vec![0; 512 * 10];
    
    // Write in large chunks for speed.
    for i in 0..2880_u16/10 {
        let pointer: DiskPointer = DiskPointer {
            disk: 12345_u16,
            block: i * 10,
        };
        
        super::write::write_large_direct(disk, &ten_blank_blocks, pointer)?;
        
        let percent = (((i + 1) * 10) as f32 / 2880_f32) * 100.0;
        println!("{percent:.1}%...");
    }
    println!("Wipe complete.");
    
    Ok(())
}