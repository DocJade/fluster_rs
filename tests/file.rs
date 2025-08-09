use std::{error::Error, ffi::OsStr, thread, time::Duration};

use log::{error, info};
use rand::{random, rng, rngs::ThreadRng, Rng};
// We want to see logs while testing.
use test_log::test;

use crate::test_common::test_mount_options; 
pub mod test_common;

#[test]
// Make a small file (512 bytes)
fn make_file_small() {
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

    // Test dir
    let mut test_dir = mount_point.path().to_path_buf();
    test_dir.push("test");

    // Make some content
    let mut random: ThreadRng = rng();
    let mut bytes: [u8; 512] = [0_u8; 512];
    random.fill(&mut bytes);

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let write_result = std::fs::write(&test_dir, bytes);
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it was deleted
    assert!(write_result.is_ok());
}


#[test]
// Make a large file (8MB)
fn make_file_large() {
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

    // Test dir
    let mut test_dir = mount_point.path().to_path_buf();
    test_dir.push("test");

    // Make some content
    let mut random: ThreadRng = rng();
    let mut bytes: [u8; 1024*1024*8] = [0_u8; 1024*1024*8];
    random.fill(&mut bytes);

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let write_result = std::fs::write(&test_dir, bytes);
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it was deleted
    assert!(write_result.is_ok());
}

#[test]
// Make and read file, make sure the contents match. (512 bytes)
fn make_and_read_file_small() {
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

    // Test dir
    let mut test_dir = mount_point.path().to_path_buf();
    test_dir.push("test");

    // Make some content
    let mut random: ThreadRng = rng();
    let mut bytes: [u8; 512] = [0_u8; 512];
    random.fill(&mut bytes);

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let write_result = std::fs::write(&test_dir, bytes);

    // Now read it back in
    let read_result = std::fs::read(&test_dir);

    // Does it match?
    let matched: bool;
    if let Ok(ref read) = read_result {
        matched = *read == bytes.to_vec();
    } else {
        matched = false;
    }
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it matched.
    assert!(write_result.is_ok());
    assert!(read_result.is_ok());
    assert!(matched);
}


#[test]
// Make and read file, make sure the contents match. (512 bytes)
fn make_and_read_file_large() {
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

    // Test dir
    let mut test_dir = mount_point.path().to_path_buf();
    test_dir.push("test");

    // Make some content
    let mut random: ThreadRng = rng();
    let mut bytes: [u8; 1024*1024*8] = [0_u8; 1024*1024*8];
    random.fill(&mut bytes);

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let write_result = std::fs::write(&test_dir, bytes);

    // Now read it back in
    let read_result = std::fs::read(&test_dir);

    // Does it match?
    let matched: bool;
    if let Ok(ref read) = read_result {
        matched = *read == bytes.to_vec();
    } else {
        matched = false;
    }
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it matched.
    assert!(write_result.is_ok());
    assert!(read_result.is_ok());
    assert!(matched);
}

#[test]
// Make a file and rename it
fn move_file() {
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

    let mut test_dir = mount_point.path().to_path_buf();
    test_dir.push("test");
    let mut moved_dir = mount_point.path().to_path_buf();
    moved_dir.push("moved");
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let content: [u8; 512] = [0_u8; 512];
    let write_result = std::fs::write(&test_dir, content);

    // Rename it
    let rename_result = std::fs::rename(&test_dir, &moved_dir);

    // Does it exist?
    let moved: bool = std::fs::exists(&moved_dir).unwrap();
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it moved.
    assert!(write_result.is_ok());
    assert!(rename_result.is_ok());
    assert!(moved);
}

#[test]
// Delete a file
fn delete_file() {
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

    let mut test_file = mount_point.path().to_path_buf();
    test_file.push("test");
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test file
    let content: [u8; 512] = [0_u8; 512];
    let write_result = std::fs::write(&test_file, content);

    // Delete it
    let delete_result = std::fs::remove_file(&test_file);

    // Does it exist?
    let removed: bool = std::fs::exists(&test_file).unwrap();
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it moved.
    assert!(write_result.is_ok());
    assert!(delete_result.is_ok());
    assert!(!removed);
}