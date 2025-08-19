// Restore a disk from a backup.

use rprompt::prompt_reply;

/// Returns true if the entire disk was re-created successfully.
/// 
/// Assumes the drive is empty when called.
pub fn restore_disk(number: u16) -> bool {
    println!("Begining restore of disk `{number}`.");

    // Get a new blank disk.
    let _ = prompt_reply(format!("Please insert a brand new, blank disk that will become the new disk {number}, then press enter."));

    // Find the disk in the backup folder

    // Copy the entire contents of that backup to the new disk
    todo!()
}