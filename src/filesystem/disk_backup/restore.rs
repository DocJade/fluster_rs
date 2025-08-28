// Restore a disk from a backup.

use std::io::{
    Read,
    Seek
};

use crate::{pool::disk::{
    drive_struct::{
        DiskType,
        FloppyDrive
    },
    generic::{
        disk_trait::GenericDiskMethods,
        generic_structs::pointer_struct::DiskPointer
    }
}, tui::prompts::TuiPrompt};

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
    println!("Opening backup file...");
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
                    println!("Fail. Out of retries.");
                    return false;
                } else {
                    println!("Fail, trying again...");
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
    println!("Reading backup data...");
    loop {
        if backed_up.rewind().is_ok() && backed_up.read_to_end(&mut bytes).is_ok() {
            // All good.
            break
        }
        if tries == 5 {
            // cooked.
            println!("Fail. Out of retries.");
            return false;
        }
        println!("Fail, trying again...");
        bytes.clear();
        tries += 1;
        continue;
    };

    // Copy the entire contents of that backup to the new disk.
    // This disk number does not actually matter, since it's only needed
    // when virtual disks are being used.
    let mut tries = 1_u8;
    let mut disk: DiskType;
    println!("Opening disk...");
    loop {
        if let Ok(opened) = FloppyDrive::open_direct(number) {
            // good.
            disk = opened;
            break
        }
        if tries == 5 {
            // cooked.
            println!("Fail. Out of retries.");
            return false;
        }
        println!("Fail, trying again...");
        tries += 1;
        continue;
    }

    // Write the entire disk in one go. We give it 5 chances to work before giving up.
    let mut tries = 1_u8;
    println!("Writing...");
    loop {
        // Yes i know the clone is stinky.
        if disk.unchecked_write_large(bytes.clone(), DiskPointer { disk: 12321, block: 0 }).is_ok() {
            break;
        } else if tries == 5 {
            // Cooked.
            println!("Fail. Out of retries.");
            return false;
        } else {
            println!("Fail, trying again...");
            tries += 1;
        }
    };

    // Disk written!
    println!("Disk restored!");
    true
}