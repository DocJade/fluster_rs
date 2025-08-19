// Methods that are generic across all types of disk.

// Using the floppy drive interface should work like this:
// Request a disk, get back a DiskType that matches the number provided.

// Imports

use log::error;
use log::trace;
use log::warn;

use crate::error_types::conversions::CannotConvertError;
use crate::error_types::critical::CriticalError;
use crate::error_types::critical::RetryCapError;
use crate::error_types::drive::DriveError;
use crate::error_types::drive::DriveIOError;
use crate::error_types::drive::WrappedIOError;
use crate::helpers::hex_view::hex_view;
use crate::pool::disk::blank_disk::blank_disk_struct::BlankDisk;
use crate::pool::disk::drive_struct::DiskBootstrap;
use crate::pool::disk::generic::block::block_structs::RawBlock;
use crate::pool::disk::generic::disk_trait::GenericDiskMethods;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
// The cache is NOT allowed in here at all, since any writes happen through the cache regardless.
// Thus if we are loading in a disk, this is a real swap.
// use crate::pool::disk::generic::io::cache::cache_io::CachedBlockIO;
use crate::pool::disk::generic::io::read::read_block_direct;

use crate::pool::disk::generic::io::wipe::destroy_disk;
use crate::pool::disk::standard_disk::standard_disk_struct::StandardDisk;

use crate::pool::disk::pool_disk::pool_disk_struct::PoolDisk;

use crate::filesystem::filesystem_struct::FLOPPY_PATH;
use crate::filesystem::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::disk::unknown_disk::unknown_disk_struct::UnknownDisk;
use crate::pool::pool_actions::pool_struct::GLOBAL_POOL;

use super::drive_struct::DiskType;
use super::drive_struct::FloppyDrive;

use std::fs::File;
use std::fs::OpenOptions;
use std::panic::Location;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

// Disk tracking global.

// To better count disk swaps, we need to know what the most recently opened disk was
static CURRENT_DISK_IN_DRIVE: AtomicU16 = AtomicU16::new(u16::MAX);

// Implementations

/// Various operations on the underlying Disk.
/// This is meant to be high level, just enough to get to the disk type below.
impl FloppyDrive {
    /// Open the disk currently in the drive, regardless of disk type.
    /// This should only be used when initializing the pool. Use open() instead.
    pub fn open_direct(disk_number: u16) -> Result<DiskType, DriveError> {
        // This function does not create disks.
        open_and_deduce_disk(disk_number, false)
    }

    /// Opens a specific disk, or waits until the user inserts that disk.
    #[deprecated ="You should be using the cache! Unless you are using this in the cache."]
    pub fn open(disk_number: u16) -> Result<DiskType, DriveError> {
        prompt_for_disk(disk_number)
    }

    /// Prompts the user for a blank floppy disk.
    pub fn get_blank_disk(disk_number: u16) -> Result<BlankDisk, DriveError> {
        prompt_for_blank_disk(disk_number)
    }

    /// Find out what disk is currently in the drive.
    pub fn currently_inserted_disk_number() -> u16 {
        CURRENT_DISK_IN_DRIVE.load(Ordering::SeqCst)
    }
}

// Functions for implementations

fn open_and_deduce_disk(disk_number: u16, new_disk: bool) -> Result<DiskType, DriveError> {
    trace!("Opening and deducing disk disk {disk_number}...");
    trace!("Is it a new disk? : {new_disk}");
    // First, we need the file to read from
    let disk_file: File = get_floppy_drive_file(disk_number, new_disk)?;

    // Now we must get the 0th block
    // We need to read a block before we have an actual disk, so we need
    // to call this function directly as a workaround.

    // This also must be called directly, since we cannot use the cache here.
    // The cache expects to not be accessed while doing flushes, which requires all
    // calls that load information about disks to not access the cache.

    // We must ignore the CRC here, since we know nothing about the disk.
    trace!("Reading in the header at block 0...");
    let header_block: RawBlock = read_block_direct(&disk_file, disk_number, 0, true)?;

    // Now we check for the magic
    trace!("Checking for magic...");
    if !check_for_magic(&header_block.data) {
        trace!("No magic, checking if its blank...");
        // The magic is missing, check if the block is empty
        if header_block.data.iter().all(|byte| *byte == 0) {
            // Block is completely blank.
            trace!("Disk is blank, returning.");
            return Ok(DiskType::Blank(BlankDisk::new(disk_file)));
        }
        // Otherwise, we dont know what kind of disk this is.
        // Its probably not a fluster disk.
        trace!("Disk was not blank, returning unknown disk...");
        return Ok(DiskType::Unknown(UnknownDisk::new(disk_file)));
    }

    // Magic exists, time to figure out what kind of disk this is.
    trace!("Disk has magic, deducing type...");
    // Bitflags will tell us.

    // Pool disk.
    // The header reads should check the CRC of the block.
    if header_block.data[8] & 0b10000000 != 0 {
        trace!("Head is for a pool disk, returning.");
        return Ok(DiskType::Pool(PoolDisk::from_header(
            header_block,
            disk_file,
        )));
    }

    // Standard disk.
    if header_block.data[8] & 0b00100000 != 0 {
        trace!("Head is for a standard disk, returning.");
        return Ok(DiskType::Standard(StandardDisk::from_header(
            header_block,
            disk_file,
        )));
    }

    // it should be impossible to get here
    error!("Header of disk did not match any known disk type!");
    error!("Hexdump:\n{}", hex_view(header_block.data.to_vec()));
    error!("We cannot continue with an un-deducible disk!");
    unreachable!();
}

/// Get the path of the floppy drive
fn get_floppy_drive_file(disk_number: u16, new_disk: bool) -> Result<File, DriveError> {
    // If we are running with virtual disks enabled, we are going to use a temp folder instead of the actual disk to speed up
    // development, waiting for disk seeks is slow and loud lol.

    trace!("Locking USE_VIRTUAL_DISKS...");
    if let Some(ref path) = *USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
    {
        trace!("Attempting to access virtual disk {disk_number}...");
        trace!("Are we creating this disk? : {new_disk}");
        // Get the tempfile.
        // These files do not delete themselves.

        // if disk 0 is missing, we need to make it,
        // because the pool cannot create disk 0 without first loading itself... from disk 0.
        // This is for virtual disks, so if this fails its on the user.
        let _ = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join("disk0.fsr")).expect("You are in-charge of making virtual disks work.");

        // If the tempfile does not exist, that means `create` was never called, which is an issue.
        // This will create the disk if the correct argument is passed.

        trace!("Opening the temp disk with read/write privileges...");
        let temp_disk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(new_disk) // We will panic if the disk does not exist, unless told to create it.
            .truncate(false)
            .open(path.join(format!("disk{disk_number}.fsr")))
            .expect("Disks should be created before read.");

        // Make sure the file is one floppy big, should have no effect on pre-existing files, since
        // they will already be this size.
        trace!("Attempting to resize the temporary file to floppy size...");

        // This is for virtual disks, so if this fails its on the user.
        temp_disk_file.set_len(512 * 2880).expect("If you're using virtual disks, you should be able to resize the virtual disks.");

        trace!("Returning virtual disk.");
        return Ok(temp_disk_file);
    }

    // Get the global path to the floppy disk drive
    trace!("Locking FLOPPY_PATH...");
    let disk_path = FLOPPY_PATH
        .try_lock()
        .expect("Fluster is single threaded.")
        .clone();

    // Open the disk, or return an error from it.

    // We will try 10 times.
    // If we fail to open the floppy drive file, there's a bigger issue than this function can deal with.

    for _ in 0..10 {
        // Open the file.
        let open_attempt = OpenOptions::new().read(true).write(true).open(&disk_path);
        
        // Did that open?
        if let Ok(file) = open_attempt {
            // Cool!
            return Ok(file);
        }

        // That did not work, see if we can cast up the error
        let io_error = open_attempt.expect_err("Guard.");

        let pointer = DiskPointer {
            disk: disk_number,
            block: 0,
        };

        // Try converting that up to a DriveError
        let wrapped: WrappedIOError = WrappedIOError::wrap(io_error, pointer);
        let drive_io_error: Result<DriveIOError, CannotConvertError> = DriveIOError::try_from(wrapped);

        // Did that work?
        if let Err(err) = drive_io_error {
            // Looks like we need to handle this ourselves.
            match err {
                CannotConvertError::MustRetry => {
                    // Look's like we're trying again!
                    continue;
                },
            }
        }

        // The conversion worked, can we get it up to a DriveError?
        let drive_error: Result<DriveError, CannotConvertError> = DriveError::try_from(drive_io_error.expect("Guard."));

        // Did that also work?
        if let Err(err) = drive_io_error {
            match err {
                CannotConvertError::MustRetry => {
                    continue;
                },
            }
        }
        
        // The conversion worked! Throw it upwards.
        return Err(drive_error.expect("Guard."));
    };

    drop(disk_path);

    // We've failed 10 times. Nothing we can do.
    // We can probably recover for this assuming the critical handler can either rebuild the disk
    // or somehow make it writable again
    CriticalError::OutOfRetries(RetryCapError::CantOpenDisk).handle();
    // If that works, recurse, we should be able to get the file now.
    get_floppy_drive_file(disk_number, new_disk)
}

/// Look for the magic "Fluster!" string.
pub fn check_for_magic(block_bytes: &[u8]) -> bool {
    // is the "Fluster!" magic present?
    block_bytes[0..8] == *"Fluster!".as_bytes()
}

/// Prompt user to insert the disk we want.
/// If the disk is already in the drive, no prompt will happen.
/// Will error out for non-wrong disk related issues.
/// This function does not disable the CRC check, you must use open() if you are ignoring CRC.
fn prompt_for_disk(disk_number: u16) -> Result<DiskType, DriveError> {
    trace!("Prompting for disk {disk_number}...");
    let mut is_user_an_idiot: bool = false; // Did the user put in the wrong disk when asked?
    let mut disk: DiskType;

    loop {
        // Try opening the current disk.
        // We do not create disks here.
        disk = open_and_deduce_disk(disk_number, false)?;

        // Is this the correct disk?
        let new_disk_number = disk.get_disk_number();

        // Update the current disk if needed
        let previous_disk = CURRENT_DISK_IN_DRIVE.load(Ordering::SeqCst);

        if new_disk_number != previous_disk {
            // We have swapped disks.
            CURRENT_DISK_IN_DRIVE.store(new_disk_number, Ordering::SeqCst);
            // Update the swap count
            trace!("Locking GLOBAL_POOL, updating disk swap count.");
            GLOBAL_POOL
                .get()
                .expect("single threaded")
                .try_lock()
                .expect("single threaded")
                .statistics
                .swaps += 1;
        }

        // Check if this is the right disk number
        if disk_number == new_disk_number {
            // Thats the right disk!
            trace!("Got the correct disk.");
            return Ok(disk);
        }

        warn!("Wrong disk received. Got disk {}", disk.get_disk_number());


        // This was not the right disk.
        // We should ALWAYS get the correct disk when testing.
        #[cfg(test)]
        if cfg!(test) {
            error!("Got an invalid disk during a test!");
            panic!("Test received an invalid disk!");
        }

        // Prompt user to swap disks.
        // But we don't prompt if the read failed, since we want to silently retry it.
        if is_user_an_idiot {
            println!("Wrong disk. Try again.");
        } else {
            is_user_an_idiot = true;
        }
        let _ = rprompt::prompt_reply(format!(
            "Please insert disk {disk_number}, then press enter."
        ));
    }
}

// get a blank disk
fn prompt_for_blank_disk(disk_number: u16) -> Result<BlankDisk, DriveError> {
    // Pester user for a blank disk
    let mut try_again: bool = false;

    // If we are on virtual disks, skip the initial prompt
    if !USE_VIRTUAL_DISKS
        .try_lock()
        .expect("Fluster is single threaded.")
        .is_some()
    {
        let _ = rprompt::prompt_reply(
            format!("Creating a new disk, please insert a blank disk that will become disk {disk_number}, then hit enter."),
        ).expect("Prompts should not fail.");
    }

    loop {
        if try_again {
            let action = rprompt::prompt_reply(
                "That disk is not blank. Please insert a blank disk, then hit enter. Or type \"wipe\" to forcibly wipe this disk.",
            ).expect("Prompts should not fail.");

            if action.contains("wipe") {
                // go wipe that disk
                let mut wipe_me = open_and_deduce_disk(disk_number, false)?;
                destroy_disk(wipe_me.disk_file_mut())?;
                drop(wipe_me);
            }

        }
        // we are making a new disk, so we must specify as such.
        let mut disk = open_and_deduce_disk(disk_number, true)?;
        match disk {
            // if its blank, all done
            DiskType::Blank(blank_disk) => return Ok(blank_disk),
            _ => {
                // But if the disk is not blank, 
                display_info_and_ask_wipe(&mut disk)?;
                // try again
                try_again = true;
                continue;
            }
        }
    }
}

/// Takes in a non-blank disk and displays info about it, then asks the user if they would like to wipe the disk.
/// Wipes the disk if the user asks, returns nothing.
/// Will also return nothing if the user does not wipe the disk.
pub fn display_info_and_ask_wipe(disk: &mut DiskType) -> Result<(), DriveError> {
    // This isn't a very friendly interface, but it'll do for now.

    loop {
        // Display the disk type
        println!("The disk inserted is not blank. It is of type `{disk:?}`.");
        println!("Would you like to wipe this disk?");
        let answer = rprompt::prompt_reply("y/n: ").expect("Prompts should not fail.")
            .to_ascii_lowercase()
            .contains('y');
        if answer {
            // Make absolutely sure!
            if rprompt::prompt_reply("Are you really sure? (Type \"Do as I say!\"): ").expect("Prompts should not fail.")
            .contains("Do as I say!") {
                // Wipe time!
                destroy_disk(disk.disk_file_mut()).expect("Prompts should not fail.");
                return Ok(());
            }
            println!("You've chickened out.");
            continue;
        } else {
            // No wipe.
            print!("Okay, this disk will not be wiped.");
            let _ = rprompt::prompt_reply("Please insert a different disk, then hit return.").expect("Prompts should not fail.");
            return Ok(());
        }
    }
}