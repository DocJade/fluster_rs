use std::{error::Error, thread, time::Duration};

use log::{error, info};
// We want to see logs while testing.
use test_log::test;

use crate::test_common::test_mount_options; 
pub mod test_common;

#[test]
// Try creating a directory
fn create_directory() {
    let fs = test_common::start_filesystem();
    let mount_point = test_common::get_actually_temp_dir();
    let thread_mount_path = mount_point.path().to_path_buf();
    let mount_options = test_mount_options();
    
    // fs needs to be mounted in another thread bc it blocks
    let mount_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100)); // Pause to let the debugger see the thread
        // If we dont pause, breakpoints dont work.
        // This blocks until the unmount happens.
        fuse_mt::mount(fs, &thread_mount_path, &mount_options)
    });

    let mounted_fs_path = mount_point.path().to_path_buf();
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));

    // make a new dir
    let mut new_dir = mounted_fs_path.clone();
    new_dir.push("testdir");

    info!("Attempting to create a new directory...");
    info!("It will go at `{}`.", new_dir.display());
    let creation_result = std::fs::create_dir(new_dir);
    info!("Finished attempting creation.");
    
    // See if it's there
    info!("Checking if directory exists...");
    let find_result = std::fs::read_dir(mounted_fs_path);
    let mut file_found: bool = false;
    if let Ok(items) = find_result {
        info!("Directory read succeeded, checking for the test dir...");
        for i in items {
            // Check the results
            if let Ok(good) = i {
                // is this testdir?
                let item_name = good.file_name();
                info!("found {}", item_name.display());
                if item_name == "testdir" {
                    // It exists!
                    file_found = true;
                }
                // Ignore anything that isnt the directory we are looking for.
            } else {
                // Item was an error. uh oh
                let extracted_error = i.unwrap();
                error!("Error directory item: {extracted_error:#?}");
            }
            
        }
    } else {
        error!("Reading the directory failed.");
        let read_error = find_result.unwrap_err();
        if let Some(src) = read_error.source() {
            error!("source: {src}");
        } else {
            error!("source not marked.");
        }
        error!("error: {read_error}");
        error!("kind: {}", read_error.kind());
    }

    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Do the unwrap after unmounting, so we unmount even if it failed.

    // Did the creation fail?
    if let Err(error) = creation_result {
        // why?
        error!("Folder creation failed.");
        if let Some(src) = error.source() {
            error!("source: {src}");
        } else {
            error!("source not marked.");
        }
        error!("error: {error}");
        error!("kind: {}", error.kind());
        panic!()
    }
    // Was the folder there?
    assert!(file_found, "Directory was not created, or did not show up when listed.");
}
