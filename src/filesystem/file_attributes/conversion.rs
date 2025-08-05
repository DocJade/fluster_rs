use fuse_mt::{FileAttr, FileType};
use log::debug;
use std::time::SystemTime;

use crate::{
    filesystem::file_handle::file_handle_struct::FileHandle,
    pool::disk::{
        drive_struct::FloppyDriveError,
        standard_disk::block::{
            directory::directory_struct::{
                DirectoryFlags,
                DirectoryItem
            }
        }
    }
};



// Take in a file handle and spit out its attributes.
impl TryFrom<FileHandle> for FileAttr {
    type Error = FloppyDriveError;
    
    fn try_from(value: FileHandle) -> Result<Self, Self::Error> {
        debug!("Retrieving file metadata from handle...");
        // Get the directory item
        let item: DirectoryItem = value.get_directory_item()?;
        go_get_metadata(item)
    }
}

// You can also call this on DirectoryItem
impl TryFrom<DirectoryItem> for FileAttr {
    type Error = FloppyDriveError;

    fn try_from(value: DirectoryItem) -> Result<Self, Self::Error> {
        go_get_metadata(value)
    }
}

fn go_get_metadata(item: DirectoryItem) -> Result<FileAttr, FloppyDriveError> {

    // Now for ease of implementation, we (very stupidly) ignore all file access permissions,
    // owner information, and group owner information.

    // Root owns all files (user id 0)
    // Owner is in the superuser group (group id 0)
    // All permission bits are set (very scary!) go execute a jpeg, i dont even care anymore.

    // Due to this, we also do not check any permissions on reads or writes! :D


    
    // How big is it
    let size: u64 = item.get_size()?;
    
    
    // extract the times
    let creation_time: SystemTime = item.get_created_time()?.into();
    let modified_time: SystemTime = item.get_modified_time()?.into();
    
    // "What kind of item is this?"
    // https://www.tiktok.com/@ki2myyysc6/video/7524954406438161694
    let file_kind: FileType = if item.flags.contains(DirectoryFlags::IsDirectory) {
        // "This is a directory, used for holding items in a filesystem, such as files or other directories."
        FileType::Directory
    } else {
        // "This is a file, used to store arbitrary data, it is very useful!"
        FileType::RegularFile
    };

    // Put it all together
    Ok(FileAttr {
        // Size of item in bytes.
        size,
        // Bytes div_ceil 512
        blocks: size.div_ceil(512),
        // We dont support access times.
        atime: SystemTime::UNIX_EPOCH,
        // modification time
        mtime: modified_time,
        // metadata change, not supported
        ctime: SystemTime::UNIX_EPOCH,
        // creation time
        crtime: creation_time,
        // file type
        kind: file_kind,
        // File permissions, not supported
        perm: 0b1111111111111111, // All permission bits
        // links not supported
        nlink: 0,
        // owner id, always root
        uid: 0,
        // owner group, always root
        gid: 0,
        // special id, not supported
        rdev: 0,
        // macos flags, who gaf? not me. use a real operating system /bait
        flags: 0,
    })
}