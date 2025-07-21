use std::{thread, time::Duration};

use test_log::test; // We want to see logs while testing.
mod test_common;

#[test]
// Try starting up the filesystem
fn mount_filesystem() {
    let fs = test_common::start_filesystem();
    let mount_point = test_common::get_actually_temp_dir();
    let mount_path = mount_point.path().to_path_buf();

    // fs needs to be mounted in another thread bc it blocks
    let mount_thread = thread::spawn(move || {
        easy_fuser::mount(fs, &mount_path, &[]).unwrap();
    });

    // wait for it to start...
    thread::sleep(Duration::from_millis(100));

    // Immediately unmount.
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let _ = mount_thread.join();
}