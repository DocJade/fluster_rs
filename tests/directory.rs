use std::{thread, time::Duration};

use log::{error, info};
use test_log::test;

use crate::test_common::test_mount_options; // We want to see logs while testing.
mod test_common;

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

    let mut mounted_fs_path = mount_point.path().to_path_buf();
    // Filesystem will mount itself in a folder named `fluster_test`
    mounted_fs_path.push("fluster_test");
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));

    // make a new dir
    let mut new_dir = mounted_fs_path.clone();
    new_dir.push("testdir");

    info!("Attempting to create a new directory...");
    let creation_result = std::fs::create_dir(new_dir);
    info!("Finished creating directory.");
    
    // See if it's there
    info!("Checking if directory exists...");
    let find_result = std::fs::read_dir(mounted_fs_path);
    let mut file_found: bool = false;
    if let Ok(items) = find_result {
        for i in items {
            // skip error items
            if i.is_err() {
                continue;
            }
            // is this testdir?
            let item_name = i.unwrap().file_name();
            info!("found {}", item_name.display());
            if item_name == "testdir" {
                // It exists!
                file_found = true;
            }
        }
    } else {
        error!("Reading the directory failed.");
    }

    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    creation_result.unwrap(); // Do the unwrap after unmounting, so we unmount even if it failed.
    // Was the folder there?
    assert!(file_found, "Directory was not created, or did not show up when listed.");
}
