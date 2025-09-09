// Restore a disk from a backup.

use std::{fs::File, io::{
    Read,
    Seek
}, os::unix::fs::FileExt};

use log::{debug, error, warn};

use crate::{filesystem::filesystem_struct::FLOPPY_PATH, tui::prompts::TuiPrompt};

/// Returns true if the entire disk was re-created successfully.
/// 
/// Assumes the drive is empty when called.
pub fn restore_disk(number: u16) -> bool {
    println!("Beginning restore of disk `{number}`.");

    // Get a new blank disk.
    TuiPrompt::prompt_enter(
        "Insert blank disk.".to_string(),
        format!("Please insert a brand new, blank disk that will become the new disk {number}, then press enter.\n
        WARNING: Disk will NOT be checked for blankness, this WILL destroy data if a non-blank disk is inserted!"),
        false
    );

    // Find the disk in the backup folder
    // If it't not in there, you're cooked.
    // Try opening the backup file at most 5 times.

    let mut tries = 1_u8;
    let mut backed_up: std::fs::File;
    debug!("Opening backup file...");
    loop {
        match std::fs::OpenOptions::new()
        .read(true)
        .open(format!("/var/fluster/disk_{number}.fluster_backup")) {
            Ok(ok) => {
                backed_up = ok;
                break;
            },
            Err(_) => {
                if tries == 5 {
                    // ruh roh
                    warn!("Fail. Out of retries.");
                    return false;
                } else {
                    warn!("Fail, trying again...");
                    tries += 1;
                    continue;
                }
            },
        };
    };
    
    // Now read in the entire floppy backup.
    // Again, at most 5 tries.
    
    let mut bytes: Vec<u8> = Vec::with_capacity(2880*512);
    let mut tries = 1_u8;
    debug!("Reading backup data...");
    loop {
        if backed_up.rewind().is_ok() && backed_up.read_to_end(&mut bytes).is_ok() {
            // All good.
            break
        }
        if tries == 5 {
            // cooked.
            error!("Fail. Out of retries.");
            return false;
        }
        debug!("Fail, trying again...");
        bytes.clear();
        tries += 1;
        continue;
    };

    // Copy the entire contents of that backup to the new disk.

    // We'll just dump the entire file to the block device without using our floppy handler,
    // since we cant really trust my logic hehe

    // Get the path to the floppy drive block device.
    // We'll pre-clear poison, just in case.
    FLOPPY_PATH.clear_poison();
    let block_path = if let Ok(path) = FLOPPY_PATH.lock()  {
        path.clone()
    } else {
        // well... cooked
        error!("Floppy path is poisoned!");
        return false
    };

    // Open the block device as a file
    let block_file: File = if let Ok(opened) = File::options().read(true).write(true).open(block_path) {
        opened
    } else {
        // Well, we couldn't open the floppy path. Return false.
        return false;
    };
    
    // Now write all that in
    let mut write_worked = false;
    for _ in 0..5 {
        let result = block_file.write_all_at(&bytes, 0);
        match result {
            Ok(_) => {
                // write finished!
                write_worked = true;
                break
            },
            Err(err) => {
                // That didn't work!
                error!("Writing to the drive failed!");
                error!("{err:#?}");
                continue;
            },
        }
    }

    // Did that work?
    if write_worked {
        // Disk written!
        debug!("Disk restored!");
        true
    } else {
        // Well shoot.
        error!("Disk restore failed!");
        false
    }
}