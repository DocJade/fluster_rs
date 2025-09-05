// Update the backup disk with new contents.

// The path that the disk backups go in /var/fluster

// Porting fluster? Then you've gotta update this for sure.

use std::os::unix::fs::FileExt;

use log::error;

use crate::{filesystem::filesystem_struct::WRITE_BACKUPS, pool::disk::generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer}};

pub(crate) fn update_backup(block: &RawBlock) {
    // Ignore backups if needed.
    if let Some(ian_the_bool) = WRITE_BACKUPS.get() {
        if !ian_the_bool {
            // Skip, backups are disabled.
            return
        }
    } else {
        // The backups flag hasn't been set up, which should be impossible, but we'll just return
        return
    }

    // Make the backup folder if it does not exist yet
    if std::fs::create_dir_all("/var/fluster").is_err() {
        // Unable to create the folders and such.
        error!("Fluster needs to be able to create/use /var/fluster for disk backups.");
        error!("We cannot continue without backups. Shutting down. If you are unable to use");
        error!("backups, set the flag.");
        panic!("Unable to update backups!"); // we panic here, since we still want to flush the disks.
    }

    // Open or create the backup file for the disk
    let disk_path: String = format!("/var/fluster/disk_{}.fluster_backup", block.block_origin.disk);
    let backup_file = if let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(disk_path) {
        file
    } else {
        // Cannot open the backup file, we're cooked.
        error!("Fluster was unable to create or open one of its backup files, if you see this spamming your logs, its probably chronic.");
        error!("Fix your permissions, or backups wont work!");
        // pretend we did the backup, crashing is worse.
        return
    };

    // Write in that block. We will try twice at maximum.
    for _ in 0..2 {
        if backup_file.write_all_at(&block.data, block.block_origin.block as u64 * 512).is_err() {
            // That did not work. Crap.
            // We'll try again.
        } else {
            // That worked!
            return
        }
    }

    // We couldn't update the file.
    error!("Fluster failed to write to a backup for one of the disks, if you see this spamming your logs, its probably chronic.");
    error!("You should investigate!");

}

pub(crate) fn large_update_backup(start: DiskPointer, data: &[u8]) {
    // Ignore backups if needed.
    if let Some(ian_the_bool) = WRITE_BACKUPS.get() {
        if !ian_the_bool {
            // Skip, backups are disabled.
            return
        }
    } else {
        // The backups flag hasn't been set up, which should be impossible, but we'll just return
        return
    }

    // Open or create the backup file for the disk
    let disk_path: String = format!("/var/fluster/disk_{}.fluster_backup", start.disk);
    let backup_file = if let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(disk_path) {
        file
    } else {
        // Cannot open the backup file, we're cooked.
        error!("Fluster was unable to create or open one of its backup files, if you see this spamming your logs, its probably chronic.");
        error!("Fix your permissions, or backups wont work!");
        // pretend we did the backup, crashing is worse.
        return
    };

    // Write in that block. We will try twice at maximum.
    for _ in 0..2 {
        if backup_file.write_all_at(data, start.block as u64 * 512).is_err() {
            // That did not work. Crap.
            // We'll try again.
        } else {
            // That worked!
            return
        }
    }

    // We couldn't update the file.
    error!("Fluster failed to write to a backup for one of the disks, if you see this spamming your logs, its probably chronic.");
    error!("You should investigate!");
}