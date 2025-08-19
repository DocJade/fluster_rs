// Update the backup disk with new contents.

// The path that the disk backups go in /var/fluster

// Porting fluster? Then you've gotta update this for sure.

use std::os::unix::fs::FileExt;

use crate::{filesystem::filesystem_struct::WRITE_BACKUPS, pool::disk::generic::{block::block_structs::RawBlock, generic_structs::pointer_struct::DiskPointer}};

pub(crate) fn update_backup(block: &RawBlock) {
    // Ignore backups if needed.
    if !*WRITE_BACKUPS.get().expect("Should be set.") {
        // Skip, backups are disabled.
        return
    }

    // Go to the backup folder.
    std::fs::create_dir_all("/var/fluster").expect("Fluster needs to be able to create/use /var/fluster for disk backups.");

    // Open or create the backup file for the disk
    let disk_path: String = format!("/var/fluster/disk_{}.fluster_backup", block.block_origin.disk);
    let backup_file = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(disk_path).expect("Fluster needs to use its backup files.");

    // Write in that block
    backup_file.write_all_at(&block.data, (block.block_origin.block*512).into()).expect("Updating backups must work.");
}

pub(crate) fn large_update_backup(start: DiskPointer, data: &[u8]) {
    // Ignore backups if needed.
    if !*WRITE_BACKUPS.get().expect("Should be set.") {
        // Skip, backups are disabled.
        return
    }

    std::fs::create_dir_all("/var/fluster").expect("Fluster needs to be able to create/use /var/fluster for disk backups.");
    let disk_path: String = format!("/var/fluster/disk_{}.fluster_backup", start.disk);
    let backup_file = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(disk_path).expect("Fluster needs to use its backup files.");

    // Write in that block
    backup_file.write_all_at(data, (start.block*512).into()).expect("Updating backups must work.");
}