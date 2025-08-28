// Critical errors are errors that we cannot recover from without some sort of higher intervention.
// Returning this error type means you've done all you possibly can, and need saving at a higher level, or
// we are in a unrecoverable state.

use std::{fs::OpenOptions, os::unix::fs::FileExt, path::{Path, PathBuf}, process::exit};

use thiserror::Error;
use log::{error, warn};

use crate::{error_types::drive::InvalidDriveReason, filesystem::{disk_backup::restore::restore_disk, filesystem_struct::FLOPPY_PATH}, pool::disk::generic::generic_structs::pointer_struct::DiskPointer, tui::prompts::TuiPrompt};

#[derive(Debug, Clone, Copy, Error, PartialEq)]
/// Use this error type if an error happens that you are unable to
/// recover from without intervention.
/// 
/// Creating critical errors is a last resort. Whatever error that was causing
/// your failure must be passed in.
pub enum CriticalError {
    #[error("The floppy drive is inaccessible for some reason.")]
    DriveInaccessible(InvalidDriveReason),
    /// Set the bool to true if Fluster could reasonably continue even after failing this operation.
    #[error("We've retried an operation too many times. Something must be wrong.")]
    OutOfRetries(RetryCapError) // Keep track of where we ran out of retries.
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// When you run out of retries on an operation, its useful to know what kind of issue was occurring.
pub enum RetryCapError {
    /// Opening the disk is repeatedly failing
    CantOpenDisk,
    /// Attempting to write a block is repeatedly failing.
    CantWriteBlock,
    /// Attempting to read a block is repeatedly failing.
    CantReadBlock,
}

//
// =========
// Attempt to recover
// =========
//

impl CriticalError {
    /// Try to recover from a critical error.
    /// 
    /// Returns nothing, since if recovery fails, fluster has shut down.
    /// If this function completes successfully, you can re-attempt the operation that resulted in the critical error.
    /// This should only be called once per operation, if you are consistently calling attempt_recovery, there is a deeper
    /// issue that you must address.
    pub(crate) fn handle(self) {
        go_handle_critical(self)
    }
}


fn go_handle_critical(error: CriticalError) {

    // Critical recovery is not allowed in tests.
    if cfg!(test) {
        panic!("Tried to recover from a critical error! {error:#?}");
    }

    let mitgated = match error {
        CriticalError::DriveInaccessible(invalid_drive_reason) => handle_drive_inaccessible(invalid_drive_reason),
        CriticalError::OutOfRetries(reason) => handle_out_of_retries(reason),
    };


    // If that worked, the caller that caused this critical to be thrown should be able to
    // complete whatever operation they need.
    if mitgated {
        return
    }

    // None of that worked. We must give up.
    // .o7
    println!("Critical error recovery has failed.");
    println!("{error:#?}");
    println!("Fluster! has encountered an unrecoverable error, and must shut down.\nGoodbye.");
    exit(-1);
}



//
// Sub-type handlers
//


// Returns true if mitigation succeeded.
fn handle_drive_inaccessible(reason: InvalidDriveReason) -> bool {
    match reason {
        InvalidDriveReason::NotAFile => {
            // A non-file cannot be used as a floppy disk
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::PermissionDenied => {
            // Need to be able to do IO obviously.
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::Networking => {
            // Cant use network drives
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::ReadOnly => {
            // need to write sometimes.
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::NotSeekable => {
            // Floppy drives must be seekable.
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::InvalidPath => {
            // Need to be able to get to the drive
            inform_improper_floppy_drive()
        },
        InvalidDriveReason::UnsupportedOS => {
            // Homebrew OS maybe? We don't use that many
            // file operations, certainly not many unusual ones, thus
            // this shouldn't happen on normal platforms.
            error!("Simple file-based IO is marked as unsupported by your operating system.");
            error!("I'm assuming you're using a non-standard Rust build target / OS destination.");
            error!("Obviously I cannot support that. If you really want to use Fluster (why?), you'll have to");
            error!("update Fluster to make it compatible with your system/setup. Good luck!");
            exit(-1);
        },
        InvalidDriveReason::NotFound => {
            // Maybe the drive is tweaking?
            // Ask the user if they wanna do troubleshooting.
            loop {
                let response = TuiPrompt::prompt_input(
                    "Floppy drive error.".to_string(),
                    "The floppy drive was not found, would you like to retry, or start troubleshooting?\n
                    (R)etry / (T)roubleshoot".to_string(),
                    true
                );
                if response.starts_with('r') {
                    // User just wants to the retry.
                    // retrun true, since we've "done all we can"
                    return true;
                } else if response.starts_with('t') {
                    // Since the drive is not found, we will first
                    return troubleshooter();
                }
            }
            
        },
    }
}

/// Returns true if mitigation succeeded.
/// 
/// yes this is the same as the other handler, but whatever
fn handle_out_of_retries(reason:RetryCapError) -> bool {
    match reason {
        RetryCapError::CantOpenDisk => {
            // Run the troubleshooter
            troubleshooter()
        },
        RetryCapError::CantWriteBlock => troubleshooter(),
        RetryCapError::CantReadBlock => troubleshooter(),
    }
}


//
// User guided troubleshooting
//

/// Returns true if we were able to pinpoint the issue and resolve it.
fn troubleshooter() -> bool {
    // Inform the user that the troubleshooter is running.
    println!("Fluster is troubleshooting, please wait...");

    // Do the easiest things first, preferably ones that do not involve interaction.

    // Run the disk checker.

    // If that passes, we now know:
    // - Every block on the disk is readable
    // - Every block on the disk is writable.
    // - The drive is connected properly and is working.
    
    // If all of that is working, troubleshooting is done, since we did not find any issues.
    // But this is suspicious. Why did the troubleshooter get called when everything is working?
    if check_disk() {
        TuiPrompt::prompt_enter(
            "Strange...".to_string(),
            "Troubleshooter unexpectedly found nothing wrong.\n
            Suggestion: You should cancel all file operations and unmount Fluster to flush everything
            to disk, just in case.\n
            If you are already in the process of unmounting, good luck!".to_string(),
            true
        );
        return true;
    }
    
    // Something is wrong with the disk or the drive. We will now walk through the
    // fastest and easiest options first.
    
    // Ask the user to re-seat the disk.
    TuiPrompt::prompt_enter(
        "Troubleshooting: Re-seat floppy.".to_string(),
        "Please eject the floppy disk, then re-insert it.\n
        If the disk is currently spinning, please wait a moment to see if it will stop spinning before
        performing the ejection. If the disk continues to spin regardless, proceed with ejection.".to_string(),
        true
    );
    
    // Maybe re-seating was all we needed?
    if check_disk() {
        // Neat.
        troubleshooter_finished();
        return true
    }

    // Now we know for sure that either the disk is dead, or the drive is not working.

    // Let's try another disk, that'll let us narrow it down if the disk was bad.
    TuiPrompt::prompt_enter(
        "Troubleshooting: Different disk.".to_string(),
        "Please swap disks to any known good disk. Remember which disk was removed.".to_string(),
        true
    );

    // Run the check then have the user put the possibly bad disk back in for continuity.
    let disk_bad = check_disk();

    TuiPrompt::prompt_enter(
        "Troubleshooting: Return disk.".to_string(),
        "Please swap back to the disk you previously removed.".to_string(),
        true
    );

    // Now, if the known good disk passed the disk check, we know that it's the drive that is having issues.
    // Otherwise, the disk is bad.

    if disk_bad {
        // Bummer, we need to replace this disk.
        do_disk_restore();

        // Disk has been restored.
        return true;
    }

    // Disk wasn't bad, so the drive must be the issue.

    // Try un-plugging and plugging it back in lmao.

    loop {
        do_remount();

        if check_disk() {
            // Remounting fixed it.
            troubleshooter_finished();
            return true;
        };

        // That failed. Retry?
        let prompted = TuiPrompt::prompt_input(
            "Troubleshooting: Try again?".to_string(),
            "Check disk is still failing.\n
            This is our last troubleshooting step before completely giving up.\n
            Would you like to try re-mounting again, or throw in the towel?\n
            (Y)es/(G)ive up".to_string(),
            true
        );

        if prompted.to_ascii_lowercase().contains('g') {
            // user gives up.
            break
        }
    }

    // Remounting did not work, and the user has given up.
    TuiPrompt::prompt_enter(
        "Troubleshooting failed. :(".to_string(),
        "Troubleshooting has failed. No fix that was attempted worked.\n
        All of the disks are backed up to the backup directory. No data should be lost, although there might be partially written data.\n
        Worst comes to worst, you can re-image all of your disks from backups.\n
        Before restoring those disks though, make sure to back-up the backups, since they might be slightly corrupt.".to_string(),
        false
    );
    return false;
}


//
// Troubleshooting actions
//

/// Read every block on the disk to determine if the disk is bad.
/// 
/// This may take a while.
/// 
/// Returns true if every block was read and written correctly.
fn check_disk() -> bool {
    println!("Checking if the disk and drive are working...");
    // Just loop over all of the blocks and try reading them.
    // We need to do it manually ourselves since we dont want
    // to throw another critical error while handling another one.

    // Open the disk currently in the drive.
    let disk_path = FLOPPY_PATH
        .try_lock()
        .expect("Fluster is single threaded.")
        .clone();
    
    // Read the entire thing in one go
    println!("Open floppy drive...");
    let disk_file = match OpenOptions::new().read(true).write(true).open(&disk_path) {
        Ok(ok) => ok,
        Err(error) => {
            // There is something wrong with reading in the drive, which would imply that
            // the drive is inaccessible or something. We cannot resolve here.
            println!("Failed to open drive.");
            println!("{error:#?}");
            return false;
        },
    };
    println!("Ok.");
    
    // Now read in the entire disk.
    println!("Reading entire disk...");
    let mut whole_disk: Vec<u8> = vec![0; 512*2880];
    let _ = disk_file.sync_all();
    let read_result = disk_file.read_exact_at(&mut whole_disk, 0);
    let _ = disk_file.sync_all();
    
    // If that failed at all, checking the disk is bad either due to the drive, or the disk.
    if let Err(error) =  read_result {
        // Read failed. Something is up.
        println!("Fail.");
        println!("{error:#?}");
        return false;
    };
    println!("Ok.");
    
    // Now we write the entire disk back again to see if every block accepts writes.
    println!("Writing entire disk...");
    let _ = disk_file.sync_all();
    let write_result = disk_file.write_all_at(&whole_disk, 0);
    let _ = disk_file.sync_all();
    
    // Did the write work?
    if let Err(error) = write_result {
        // nope
        println!("Fail.");
        println!("{error:#?}");
        return false;
    };
    println!("Ok.");
    println!("Disk and drive appear to be working correctly.");
    true
}


/// Some actions might change the path to the floppy disk drive, we need to let the user update that
/// if they need.
fn update_drive_path() {
    // what's the new path
    let new_path: std::path::PathBuf;
    loop {
        let possible = TuiPrompt::prompt_input(
            "Troubleshooting: Path change.".to_string(),
            "If the path to the floppy drive has changed due to the re-mount, please
            enter the new path. Otherwise hit enter.".to_string(),
            false
        );
        let could_be = PathBuf::from(possible);
        let maybe = match could_be.canonicalize() {
            Ok(ok) => ok,
            Err(err) => {
                // what
                TuiPrompt::prompt_enter(
                    "Invalid path.".to_string(),
                    format!("Unable to canonicalize path. Please provide a valid path.\n\n{err:#?}"),
                    false
                );
                continue;
            },
        };

        if std::fs::exists(&maybe).unwrap_or(false) {
            // Good.
            new_path = maybe;
            break
        } else {
            TuiPrompt::prompt_enter(
                "Invalid path.".to_string(),
                format!("Unable to either open path, or confirm it exists. Please provide a valid path."),
                false
            );
            continue;
        }
    }

    // Set that new path
    *FLOPPY_PATH.try_lock().expect("Fluster is single threaded.") = new_path;
}




//
// User actions
//



/// Ask the user to remount the floppy drive.
fn do_remount() {
    TuiPrompt::prompt_enter(
        "Troubleshooting: Remount drive.".to_string(),
        "Please re-mount the floppy drive.\n
        You can find more information about remounting in the README.\n
        Press enter after you have finished re-mounting the drive.".to_string(),
        false
    );
    // This might have changed the path to the floppy drive.
    update_drive_path();
}

/// Inform the user that the disk needs to be re-created.
/// 
/// Make sure to put the bad disk back in the drive beforehand so the user
/// knows what disk to discard.
fn do_disk_restore() {
    TuiPrompt::prompt_enter(
        "Troubleshooting: Bad disk.".to_string(),
        "The troubleshooter has determined that the disk currently within the drive is bad.\n
        This disk will need to be re-created.".to_string(),
        false
    );

    // Now start the restore.
    let mut failure = false;
    loop {
        if failure {
            // We tried restoring the disk, but the restore failed.
            // Tell the user and ask if they want to attempt to restore to
            // the same disk again, or have them put in a new disk.
            TuiPrompt::prompt_enter(
                "Restoration: Failure.".to_string(),
                "Restoring disk has failed. Restoring can be retried though.\n
                If you would like to attempt restoring to the same disk that you inserted previously,
                leave it in the drive, and ignore the message about swapping disks.\n
                You can re-try as many times as you would like, but if the new disk continues to fail, you
                should try using another disk to restore onto.\n
                If you just cannot seem to restore to a new disk, idk man you're cooked lmao good luck bozo.".to_string(),
                false
            );
        }

        println!("");
        
        // Pull out the bad one, disk restore needs an empty drive.
        // We need to know what disk it was.
        let disk_number: u16;
        loop {
            let to_convert = TuiPrompt::prompt_input(
                "Restoration: New disk.".to_string(),
                "Please remove the bad disk currently inserted in the drive, then
                enter it's disk number.".to_string(),
                false
            );
            if let Ok(number) = to_convert.parse::<u16>() {
                disk_number = number;
                break;
            }
            TuiPrompt::prompt_enter(
                "Restoration: Bad number.".to_string(),
                "Parsing error, please try again. Only enter the number of the disk.".to_string(),
                false
            );
        }

        // Now restore that disk.
        if restore_disk(disk_number) {
            // restore worked!
            return
        }
        // Restoration failed, it can be retried.
        failure = true;
    }
}

//
// User information
//

fn inform_improper_mount_point() -> ! {
    TuiPrompt::prompt_enter(
        "Bad mount point.".to_string(),
        "The point where you have tried to mount fluster is invalid for some reason.\n
        Please re-confirm that the mount point is valid, then re-run fluster. Good luck!".to_string(),
        true
    );
    exit(-1)
}

fn inform_improper_floppy_drive() -> bool {
    // We cannot use this floppy drive.

    // First check if the user has inserted a write-protected disk
    loop {
        let prompted: String = TuiPrompt::prompt_input(
            "Troubleshooting: Write protected.".to_string(),
            "Please remove the floppy disk from the drive, and confirm that it is not set to read-only.\n\n
            Was the disk set to read-only? \"yes\"/\"no\"".to_string(),
            false
        );
        if prompted.contains('y') {
            // Whoops!
            TuiPrompt::prompt_enter(
                "Troubleshooting finished.".to_string(),
                "Cool! That means we do not need to shut down.
                Please set the disk to read/write, and insert it back into the drive.\n
                Please make sure you do not insert write protected disks in the future, or the troubleshooter will start again.".to_string(),
                false
            );
            return true;
        } else if prompted.contains('n') {
            // Well crap.
            break
        };
    }

    // Disk was not write protected. Drive is bad.
    TuiPrompt::prompt_enter(
        "Troubleshooting failed.".to_string(),
        "Fluster is unable to access the floppy disk from your floppy drive.\n
        Please make sure that the path you provided for the drive is:\n
        - Valid\n
        - A file, and not a directory\n
        - Accessible by your current user\n
        - Is not over the network\n
        - Is not mounted as read-only\n\n
        Fluster will now exit, since operating without a drive is not possible.".to_string(),
        true
    );
    exit(-1);
}

// Helper just do dedupe
fn troubleshooter_finished() {
    TuiPrompt::prompt_enter(
        "Troubleshooting succeeded!".to_string(),
        "Troubleshooting finished successfully.".to_string(),
        false
    );
}