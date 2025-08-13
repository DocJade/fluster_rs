use std::{error::Error, ffi::OsStr, thread, time::Duration};

use log::{debug, error, info};
use rand::{rngs::ThreadRng, seq::SliceRandom};
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

#[test]
// make dir, and some test items. list items.
fn enter_and_list_directory() {
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
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));

    // Make test folder
    mounted_fs_path.push("test");
    std::fs::create_dir(&mounted_fs_path).unwrap();

    // Make a directory to look for.
    let mut hidden_folder_path = mounted_fs_path.clone();
    hidden_folder_path.push("hidden");

    std::fs::create_dir(hidden_folder_path).unwrap();

    // Read test folder
    let result = std::fs::read_dir(&mounted_fs_path).unwrap();

    let mut found: bool = false;
    for i in result {
        if i.unwrap().file_name() == OsStr::new("hidden") {
            found = true;
        }
    }

    
    // cleanup
    thread::sleep(Duration::from_millis(500));
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.
    assert!(found);
}


#[test]
// Make sure the dot directory exists and refers to the parent when listing.
fn check_for_dot() {
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
    
    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test folder
    mounted_fs_path.push("test");
    std::fs::create_dir(&mounted_fs_path).unwrap();

    // Make marker folder
    let mut mark_applier = mounted_fs_path.clone();
    mark_applier.push("hello_everybody");
    std::fs::create_dir(mark_applier).unwrap();

    // Check listing the dot is the same as listing the parent.
    let mut dotted = mounted_fs_path.clone();
    dotted.push(".");
    let dot_result = std::fs::read_dir(&dotted).unwrap();
    let parent_result = std::fs::read_dir(&mounted_fs_path).unwrap();

    // Do they match?
    // This is the grossest thing ever
    let mut any_different: bool = false;
    for i in dot_result.into_iter().zip(parent_result.into_iter()) {
        let (a, b) = i;
        if a.unwrap().path() != b.unwrap().path() {
            any_different = true;
        }
    }
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.
    assert!(!any_different);
}

#[test]
// Make a dir and move it to see if rename is working.
fn move_empty_directory() {
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
    
    // Make test folder
    std::fs::create_dir(&test_dir).unwrap();

    // move it
    std::fs::rename(&test_dir, &moved_dir).unwrap();

    // Does it exist?
    let moved: bool = std::fs::exists(&moved_dir).unwrap();
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it moved.
    assert!(moved);
}

#[test]
// Try removing a directory
fn directory_creation_and_removal() {
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

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));
    
    // Make test folder
    std::fs::create_dir(&test_dir).unwrap();

    // move it
    let deleted = std::fs::remove_dir(&test_dir);
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.

    // Make sure it was deleted
    assert!(deleted.is_ok());
}

#[test]
// Higher level renaming test
fn rename_lots_of_items() {
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

    // wait for it to start...
    thread::sleep(Duration::from_millis(500));

    // Hilariously we will just collect all the results we get so we can check
    // them all at the end after unmount.
    let mut results: Vec<std::io::Result<()>> = Vec::new();
    
    // A lot of directories
    let mut directories: Vec<String> = Vec::new();
    for i in 0..1000 {
        let mut new_dir = mount_point.path().to_path_buf();
        let dir_name = format!("dir_{i}");
        new_dir.push(dir_name.clone());
        directories.push(dir_name);
        let go = std::fs::create_dir(&new_dir);
        results.push(go);
    }

    // A lot of files
    let mut files: Vec<String> = Vec::new();
    for i in 0..1000 {
        let mut new_file = mount_point.path().to_path_buf();
        let dir_name = format!("file_{i}.txt");
        new_file.push(dir_name.clone());
        files.push(dir_name);
        // Doesn't need any particular content.
        let go = std::fs::write(new_file, [1,2,3,4]);
        results.push(go);
    }
    
    // Shuffle for fun
    let mut all_items: Vec<String> = Vec::new();
    all_items.extend(directories);
    all_items.extend(files);
    
    let mut random: ThreadRng = rand::rng();
    all_items.shuffle(&mut random);
    
    // How many do we have
    let number_made: usize = all_items.len();
    
    // Go rename all of them
    for name in all_items {
        let new_name: String = format!("new_{name}");
        let mut previous = mount_point.path().to_path_buf();
        let mut new = mount_point.path().to_path_buf();
        previous.push(name);
        new.push(new_name);
        let go = std::fs::rename(previous, new);
        results.push(go);
    }

    // Make sure the directory still contains the correct number of items. (ie we didn't duplicate anything.)
    // Listing also returns `.`, but rust (or linux?) drops it here if the directory is not empty. (in theory)
    
    // We also cant error out before the unmount, so we get a lil goofy here.
    let mut list_count: usize = 0;
    let mut list_names: Vec<String> = Vec::new();
    let list = std::fs::read_dir(&mount_point);
    if let Ok(listed) = list {
        // listing was ok
        for i in listed {
            if let Err(error) = i {
                // Something is amiss about this entry
                results.push(Err(error));
            } else {
                // i is good
                let entry = i.unwrap();
                // Skip if this is the `.` item
                if entry.file_name().to_string_lossy().into_owned() == "." {
                    // skip
                    continue;
                }
                list_count += 1;
                list_names.push(entry.file_name().to_string_lossy().into_owned());
            }
        };
    } else {
        // Listing failed
        results.push(Err(std::io::Error::other("Listing failed.")));
    }
    
    // Now, because somebody thought this was a good idea (it probably is overall, just not great for FS work) the
    // returned Result<> from filesystem operations seems to be holding references to the currently open filesystem.
    // So we have to extract everything. We will turn the errors into strings if they exist.

    // Loop over them, and only on the errors, make strings
    // Tried doing this with iter, but couldn't finish writing it bc rust analyzer kept crashing lmao
    let mut error_strings: Vec<String> = Vec::new();
    for result in &results {
        if let Err(error) = result {
            // Make a string from that
            let strung = error.to_string();
            error_strings.push(strung);
        }
    }

    // now drop the old results, we cant hold them past the unmount
    drop(results);
    
    // cleanup
    test_common::unmount(mount_point.path().to_path_buf());
    let unmount_result = mount_thread.join();
    unmount_result.unwrap().unwrap(); // Unmounting the fs should not fail.
    
    // Check all the results
    if !error_strings.is_empty() {
        // Something failed
        for string in error_strings {
            error!("{string}");
        }
        panic!();
    };

    // Make sure there are no duplicates
    list_names.sort_unstable();
    let old_list_len: usize = list_names.len();
    list_names.dedup();
    assert_eq!(old_list_len, list_names.len());

    // Every item should contain the word `new`
    assert!(list_names.iter().all(|i| i.contains("new")));

    // Every item should only contain `new` once.
    assert!(list_names.iter().all(|i| {
        i.matches("new").count() == 1
    }));
    
    // Make sure we have the correct number of items.
    assert_eq!(number_made, list_count);
}


// Renaming burn in test
#[test]
#[ignore = "Slow."]
fn rename_burn_in() {
    for _ in 0..1000 {
        rename_lots_of_items()
    }
}