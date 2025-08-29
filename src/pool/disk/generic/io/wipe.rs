// Squeaky clean!

use std::{fs::File, time::Duration};

use log::debug;

use crate::{error_types::drive::DriveError, pool::disk::generic::generic_structs::pointer_struct::DiskPointer, tui::{notify::NotifyTui, tasks::TaskType}};

/// Wipes ALL data on ALL blocks on the disk.
pub(crate) fn destroy_disk(disk: &mut File) -> Result<(), DriveError> {
    // Bye bye!
    let chunk_size: usize = 64;
    debug!("Wiping currently inserted disk...");
    let ten_blank_blocks: Vec<u8> = vec![0; 512 * chunk_size];

    // Make a new task to track disk wiping progress.
    let task_handle = NotifyTui::start_task(TaskType::WipeDisk, (2880/chunk_size) as u64);
    
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
            NotifyTui::cancel_task(task_handle);
            return Err(DriveError::TakingTooLong)
        }
        
        let percent = (((i + 1) * chunk_size) as f32 / 2880_f32) * 100.0;
        debug!("{percent:.1}%...");
        NotifyTui::complete_task_step(&task_handle);
    }
    debug!("Wipe complete.");
    NotifyTui::finish_task(task_handle);
    
    Ok(())
}