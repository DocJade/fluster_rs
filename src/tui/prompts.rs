// Gotta talk to people sometimes.

use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use log::debug;
use log::error;
use log::info;
use ratatui::style::{Style, Stylize};
use tui_textarea::TextArea;

use crate::error_types::drive::DriveError;
use crate::filesystem::filesystem_struct::FLOPPY_PATH;
use crate::{filesystem::filesystem_struct::USE_TUI, tui::notify::TUI_MANAGER};

#[derive(Debug)]
pub(crate) struct TuiPrompt<'a> {
    /// Title of the prompt
    pub(super) title: String,
    /// What the prompt is telling the user
    pub(super) content: String,
    /// Do we expect to get a string back from this prompt?
    pub(super) expects_string: bool,
    /// Where we send the response to. Even if the prompt
    /// doesn't require a response, we still use the oneshot to more
    /// easily block the caller of the prompt.
    /// 
    /// I'm calling this a callback because i feel like it.
    pub(super) callback: oneshot::Sender<String>,
    /// Should the window flash to get the user's attention?
    pub(super) flash: bool,
    /// The persistent text entry field, persists between
    /// frames so we don't have to extract input handling out
    /// to main.rs
    pub(super) text_area: tui_textarea::TextArea<'a>,
    /// Allow the caller to manually manipulate the pop-up, IE when enter is
    /// pressed, the layout code will not close the window. It's up to the caller to close
    /// the pop up by swapping in a none.
    pub(super) manual_close: bool,
}




// if the TUI is disabled, we still need to be able to prompt for input.
impl TuiPrompt<'_> {
    /// Make a new prompt for pressing enter.
    /// 
    /// This will block until the user presses enter.
    pub(crate) fn prompt_enter(title: String, content: String, flash: bool) {
        // We need the channel even if we arent getting a string back, since we wanna
        // block the caller thread without having to spin in a loop lockin stuff.
        let (response_tx, response_rx) = oneshot::channel();
        // Assemble and start the prompt.
        let prompt = TuiPrompt {
            title,
            content,
            expects_string: false,
            callback: response_tx,
            flash,
            text_area: TextArea::default(), // Not actually used.
            manual_close: false
        };

        if let Some(flag) = USE_TUI.get() {
            if !flag {
                // Tui is disabled.
                return disabled_prompt_enter(prompt);
            }
        } else {
            // USE_TUI is not set yet, it really should be at this point.
            // But since it isn't, we'll just fall back to disabled mode.
            return disabled_prompt_enter(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Wait for prompt to close
        let _ = response_rx.recv();

        // All done.
    }

    /// Make a new prompt for text input.
    /// 
    /// This will block until the user responds.
    pub(crate) fn prompt_input(title: String, content: String, flash: bool) -> String {
        // Get the channel for communicating the result of the prompt
        let (response_tx, response_rx) = oneshot::channel();


        // Assemble and start the prompt.
        // green text box
        let mut text_area = TextArea::default();
        text_area.set_style(Style::reset().on_black().green());
        let prompt = TuiPrompt {
            title,
            content,
            expects_string: true,
            callback: response_tx,
            flash,
            text_area,
            manual_close: false
        };

        if let Some(flag) = USE_TUI.get() {
            if !flag {
                // Legacy mode
                return disabled_prompt_input(prompt);
            }
        } else {
            // USE_TUI is not set yet, it really should be at this point.
            // But since it isn't, we'll just fall back to legacy prompting
            return disabled_prompt_input(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Wait for a response, and return it.
        // If we got no response for some reason, safest bet is to return nothing.
        response_rx.recv().unwrap_or_default()
    }

    /// Does not actually check what disk was swapped to, just waits for a swap to happen.
    /// 
    /// Returns an error if we were unable to detect disk swaps.
    /// 
    /// This function also assumes that the floppy drive is a block device, and checks agains /sys/ to deduce that.
    /// 
    /// Will this work on MacOS? Probably not. Def wont work on windows.
    pub(crate) fn prompt_wait_for_disk_swap(title: String, content: String, flash: bool) -> Result<(), DriveError> {

        // We dont actually care about this response at all, nothing ever gets sent
        // I'm just too lazy to remove the requirement for it right now.
        let (response_tx, _response_rx) = oneshot::channel();


        // Assemble
        let prompt = TuiPrompt {
            title,
            content,
            expects_string: false,
            callback: response_tx,
            flash,
            text_area: TextArea::default(), // Not actually used.
            manual_close: true
        };

        if let Some(flag) = USE_TUI.get() {
            if !flag {
                // Legacy mode is not supported at all here,
                // since test cases should never hit the disk swap prompt.
                panic!("Cannot swap disks without a tui!");
            }
        } else {
            // USE_TUI really must be set at this point, this is a dead path as far as i can tell
            unreachable!("Shouldn't be able to swap disks without ever setting USE_TUI");
        }

        // Get the disk path.
        let disk_path = if let Ok(guard) = FLOPPY_PATH.try_lock() {
            guard.clone()
        } else {
            // Cant lock it, chances are its poisoned.
            // Return a retry, since the caller needs to clear the poison (troubleshooter does that)
            return Err(DriveError::Retry);
        };

        // Now we put the prompt into the TUI, since if we put it in before checking the poison,
        // it would never close.
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Time to wait for a disk swap.

        // Now we need to wait for the state of the drive to transition to being empty, then full again.
        // There's probably a smarter way to do this, but I'm not smarter than the compiler anyways.
        let mut disk_in_drive: bool = get_block_device_size(&disk_path)? > 0;
        let mut previous_state: bool = disk_in_drive;
        let mut changes: u8 = 0;

        // If the drive was already empty when we checked (weird) then we need to increment changes, since
        // we're waiting for the state to change once (disk inserted) instead of twice (removal, insertion)
        if !disk_in_drive {
            changes += 1;
        }

        // Now loop till the disk has been swapped.
        // We keep track of how long this has been going, if over 2 minutes pass with no input, we'll bail out,
        // just in case we get stuck here, since we'd need to escape out to get to the troubleshooter.
        let started_waiting: Instant = Instant::now();
        while changes < 2 {
            // Check if there is a disk
            disk_in_drive = get_block_device_size(&disk_path)? > 0;
            
            // Did the state change?
            if disk_in_drive != previous_state {
                debug!("Drive state has changed.");
                // Change!
                changes += 1;
                previous_state = !previous_state;
                // Keep going, the while loop will end itself automagically.
                continue;
            }

            // No change, wait
            // half second pauses, as far as i can tell, the drive doesn't instantly know when it has a disk in it,
            // so that should be granular enough.
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // If more than 2 minutes have passed, bail
            if started_waiting.elapsed().as_secs() > 120 {
                debug!("Disk swap did not occur within 2 minutes, bailing.");
                // New prompt on screen, this will replace the old prompt.

                let (timeout_response_tx, timeout_response_rx) = oneshot::channel();
                let timeout_prompt = TuiPrompt {
                    title: "Anybody home?".to_string(),
                    content: "No disk was swapped within 2 minutes of requesting. Bailing out.".to_string(),
                    expects_string: false,
                    callback: timeout_response_tx,
                    flash: false,
                    text_area: TextArea::default(),
                    manual_close: false
                };

                // Send the timeout prompt
                loop {
                    if let Ok(mut lock) = TUI_MANAGER.lock() {
                        lock.user_prompt = Some(timeout_prompt);
                        break
                    }
                }

                // wait
                let _ = timeout_response_rx.recv().unwrap_or_default();

                // Error out.
                return Err(DriveError::TakingTooLong)
            }
        };

        // Disk has been swapped, remove the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = None;
                break
            }
        }

        // All done.
        Ok(())

    }

}

/// Abstraction for getting the size of a block device.
/// 
/// Returns how many blocks the block device has.
/// 
/// This assumes the mount point is `/dev/sd*`, if its not, we cant do anything.
/// 
/// Yes this is hacky. I'm not happy about it either, but std::fs::metadata always returns `0` on
/// block devices it seems, and I just need to get this working.
/// 
/// We look in /sys/block/sd*/size to get the size.
/// 
/// That file contains how many blocks are on that device total, and i see dmesg entries reporting
/// a block size change whenever the disk is removed, so this should work.
/// 
/// This also requires Fluster! to be ran as root/sudo lmao
/// 
/// Returns an error if the path does not exist (ie the mount point is not there.)
fn get_block_device_size(path: &Path) -> Result<u64, DriveError> {
    // inspired by http://syhpoon.ca/posts/how-to-get-block-device-size-on-linux-with-rust/
    // But I dont wanna use unsafe.

    // This code path also assumes that your floppy drives report a block size of 512.
    // Mine does at least. lol.

    // Make sure the path actually exists
    if !path.exists() {
        // Nope.
        // Just tell it to retry, which should hit the retry max count elsewhere and hit the troubleshooter up
        return Err(DriveError::Retry);
    }


    // Make sure the floppy path makes sense
    assert!(path.has_root(), "Floppy paths must be fully qualified.");
    // Must be in /dev/
    assert!(path.parent()
        .expect("Floppy path must have have a parent, since its in /dev/")
        .to_str()
        .expect("path has to be utf8 compat")
        .contains("dev"),
        "Path to the floppy drive must have a parent."
    );

    // Not gonna check if its a block device, its your problem if you dont point at one, and the expect should catch it.
    // Now we just need the end of the path. IE `/dev/sdf` -> `sdf`
    let block_device_name = path.file_name()
        .expect("There must be a file, cant use a folder as a floppy")
        .to_str()
        .expect("Should be valid UTF8");

    // Now we look into /sys/block/ to get how many blocks are on that device
    let path_to_info = PathBuf::from(format!("/sys/block/{block_device_name}/size"));
    let blocks_count_string = std::fs::read_to_string(&path_to_info).expect("If we cant access this file, fluster is doomed to fail.");

    // Now parse that into a number
    let block_count: u64 = blocks_count_string.trim().parse().expect("The size file should not contain non-number info.\n");

    // This is also a nice spot to make sure this is not bigger than a floppy drive. If it is,
    // chances are the user pointed at another disk and is gonna wipe their drive lmao, we'll save them.
    if block_count > 3000 { // A little wiggle room just in-case the drive mounts weird.
        // This is most likely not a floppy drive.
        error!("The provided block device that's supposed to be a floppy disk is too big.");
        error!("Chances are, you accidentally passed in a different drive.");
        error!("Fluster will now exit, as we don't want to accidentally wipe one of your drives.");
        error!("If you're trying to use another device as a floppy drive for some reason (unlikely), you're probably");
        error!("smart enough to disable this check.");
        panic!("Floppy drive is too big! See logs!");
    }

    // All done
    Ok(block_count)
}


// User input only works with the TUI enabled.
fn disabled_prompt_enter(prompt: TuiPrompt) {
    info!("Skipping prompt...");
    info!("Enter prompt: [{}]: {}", prompt.title, prompt.content);
}

fn disabled_prompt_input(_prompt: TuiPrompt) -> String {
    error!("You might not like TUI's, but this setting is secretly just for test cases.");
    panic!("You need to use the TUI to use fluster.");
}