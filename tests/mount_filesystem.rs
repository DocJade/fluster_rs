use std::{thread, time::Duration};

use test_log::test; // We want to see logs while testing.
use crate::test_common::test_mount_options;
mod test_common;

#[test]
// Try starting up the filesystem
fn mount_filesystem() {
    let fs = test_common::start_filesystem();
    let mount_point = test_common::get_actually_temp_dir();
    let mount_path = mount_point.path().to_path_buf();

    // fs needs to be mounted in another thread bc it blocks
    let mount_thread_result = thread::spawn(move || {
        // This blocks this thread until the unmount happens.
        fuse_mt::mount(fs, &mount_path, &test_mount_options())
    });

    // wait for it to start...
    thread::sleep(Duration::from_millis(1000));

    // Immediately unmount.
    // The mounted fs makes/lives in a folder named `fluster_test`, but we just unmount everything
    // in the folder that containers `fluster_test`
    test_common::unmount(mount_point.path().to_path_buf());
    
    // Make sure the mount actually happened.
    // Two unwraps, one for the join, one for the result of fuse_mf::mount
    mount_thread_result.join().unwrap().unwrap();
}
