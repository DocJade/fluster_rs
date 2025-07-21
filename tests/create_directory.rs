use std::{thread, time::Duration};

use test_log::test; // We want to see logs while testing.
mod test_common;

#[test]
// Try starting up the filesystem
fn create_directory() {
    let fs = test_common::start_filesystem();
    let mount_point = test_common::get_actually_temp_dir();
    let mount_path = mount_point.path().to_path_buf();

    // fs needs to be mounted in another thread bc it blocks
    let mount_thread = thread::spawn(move || {
        easy_fuser::mount(fs, &mount_path, &[]).unwrap();
    });

    // wait for it to start...
    thread::sleep(Duration::from_millis(100));

    // make a new dir
    let mut new_dir = mount_point.path().to_path_buf();
    new_dir.push("testdir");
    
    let result = std::fs::create_dir(new_dir);
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let _ = mount_thread.join();

    result.unwrap(); // Do the unwrap after unmounting, so we unmount even if it failed.
}