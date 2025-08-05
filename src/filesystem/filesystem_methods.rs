// I might blow a fuse.

// At this level of abstraction, we make calls to the Pool type. Nothing lower.

// Imports

use super::filesystem_struct::FLOPPY_PATH;
use super::filesystem_struct::FilesystemOptions;
use super::filesystem_struct::FlusterFS;
use super::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::disk::drive_struct::JustDiskType;
use crate::pool::disk::generic::generic_structs::pointer_struct::DiskPointer;
use crate::pool::disk::generic::io::cache::cache_io::CachedBlockIO;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryBlock;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryFlags;
use crate::pool::disk::standard_disk::block::directory::directory_struct::DirectoryItem;
use crate::pool::disk::standard_disk::block::inode::inode_struct::InodeTimestamp;
use crate::pool::disk::standard_disk::block::io::directory::types::NamedItem;
use crate::pool::pool_actions::pool_struct::Pool;
use log::debug;
use log::info;
use log::warn;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;

//
//
// Errors
//
//

// Error reference: https://docs.particle.io/reference/device-os/api/debugging/posix-errors/


//
//
// Implementations
//
//



// Spoofing FileAttributes
fn spoofed_file_attributes(
    file_size: u64,
    creation_time: InodeTimestamp,
    modified_time: InodeTimestamp,
    item_type: FileKind,
) -> FileAttribute {
    // Convert the times
    let mut system_creation_time: SystemTime = SystemTime::UNIX_EPOCH;
    // Add the seconds
    system_creation_time = system_creation_time
        .checked_add(Duration::from_secs(creation_time.seconds))
        .expect("Time stuff sucks, but this should be fine.");
    // Add the nanoseconds
    // This is less precise than expected, but if you need more than millisecond accuracy, you are using
    // the wrong filesystem lmao.
    system_creation_time = system_creation_time
        .checked_add(Duration::from_nanos(creation_time.nanos as u64 * 1000))
        .expect("Time stuff sucks, but this should be fine.");

    let mut system_modified_time: SystemTime = SystemTime::UNIX_EPOCH;
    system_modified_time = system_modified_time
        .checked_add(Duration::from_secs(modified_time.seconds))
        .expect("Time stuff sucks, but this should be fine.");
    system_modified_time = system_modified_time
        .checked_add(Duration::from_nanos(modified_time.nanos as u64 * 1000))
        .expect("Time stuff sucks, but this should be fine.");

    // Now for ease of implementation, we (very stupidly) ignore all file access permissions,
    // owner information, and group owner information.

    // Root owns all files (user id 0)
    // Owner is in the superuser group (group id 0)
    // All permission bits are set (very scary!)

    // Due to this, we also do not check any permissions on reads or writes! :D

    FileAttribute {
        size: file_size,                 // File size in bytes
        blocks: file_size.div_ceil(512), // Rounds up.
        atime: SystemTime::UNIX_EPOCH,   // We dont support access times.
        mtime: system_modified_time,
        ctime: SystemTime::UNIX_EPOCH, // We dont support change time, this is inode stuff.
        crtime: system_creation_time,
        kind: item_type,          // What kind of file is this
        perm: 0b1111111111111111, // All permission bits
        nlink: 0,                 // We do not support hard links.
        uid: 0,                   // Root
        gid: 0,                   // Superuser
        rdev: 0,                  // We do not support special files.
        blksize: 512,             // Preferred block size is 512 bytes.
        flags: 0,                 // easy_fuser actually completely ignores this.
        ttl: None,                // Default
        generation: None,         // We do not support generations
    }
}

fn open_response_flags() -> FUSEOpenResponseFlags {
    // There are flags we will always set when returning items, such as always using direct io to
    // prevent FUSE/linux/whatever from caching things.
    let mut flags = FUSEOpenResponseFlags::empty();
    // flags.insert(FUSEOpenResponseFlags::DIRECT_IO);
    // flags.insert(FUSEOpenResponseFlags::CACHE_DIR);
    // flags.insert(FUSEOpenResponseFlags::KEEP_CACHE);
    // flags.insert(FUSEOpenResponseFlags::NOFLUSH);
    // flags.insert(FUSEOpenResponseFlags::PASSTHROUGH);

    flags
}

//
//
// Now for the actual FUSE layer
//

//
// We dont use any file handles anywhere, we don't need them.
// But unfortunately, we still need to return a file handle, even if we dont use them.
// Even more unfortunately, this requires an unsafe operation. >:( I wanted to write
// this entire project with safe rust, but here we are, at the highest level, needing to
// use it.
//
// Very mad.
//

/// # Safety
/// This file handle is never used anywhere. At all.
fn love_handle() -> OwnedFileHandle {
    unsafe { OwnedFileHandle::from_raw(0) }
}

// One more thing to note is, since we are using PathBuf instead of inode:
// from easy_fuser/src/types/file_id_type.rs
// /// 2. `PathBuf`: Uses file paths for identification.
// ///    - Pros: Automatic inode-to-path mapping and caching.
// ///    - Cons: May have performance overhead for large file systems.
// ///    - Root: Represented by an empty string. Paths are relative and never begin with a forward slash.
//
// Thus the root directory is not `/`, it is ``.



//
// easy_fuser methods.
//

// We are using PathBufs as the unique identifier for paths instead of inode numbers, because inode numbers are scary.
impl FuseHandler<PathBuf> for FlusterFS {
    /// This does... Something, im not sure what, but we need it.
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        self.inner.as_ref()
    }

    fn get_default_ttl(&self) -> std::time::Duration {
        std::time::Duration::from_secs(10000) // we're slow okay
    }

    // There's a few things we need to tweak in the KernelConfig.
    // This should be automatically called right when the FS starts if I'm reading the docs correctly.
    //
    fn init(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        config: &mut easy_fuser::prelude::KernelConfig,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // We don't want to stall for too long while doing writes, so we will set a max
        // write size of 1MB.
        //
        // This value is in bytes.
        let _ = config
            .set_max_write(1024 * 1024)
            .expect("Max write size of 1MB is invalid?");

        // The Linux kernel (and others) has this cool feature where it expects (reasonably) that when you read
        // from a disk, chances are, you will want to keep reading more of it.
        // Based on this assumption, it will automatically read past the end of what was actually requested from
        // the disk, and keep that in a little buffer/cache so subsequent reads can skip the disk.
        //
        // In Fluster!, every additional byte you read increases the chance that the use will have to swap disks.
        // Any read-ahead could cause pointless disk swapping, since maybe that application did only need those
        // exact bytes it requested. In a normal filesystem, this would be super super stupid to turn off for
        // performance reasons, but this is Fluster! bay-be, we clown in this mf.
        //
        // In theory this shouldn't matter, since we want to be mounting Fluser! in direct-io mode to disable
        // all kernel side caching.
        //
        // This cannot be set to zero, and I cannot even find what unit this is, it might be KB?
        let _ = config
            .set_max_readahead(1)
            .expect("Checked the implementation, this requires at least 1.");

        Ok(())
    }

    // fn bmap(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     blocksize: u32,
    //     idx: u64,
    // ) -> easy_fuser::prelude::FuseResult<u64> {
    //     self.get_inner().bmap(req, file_id, blocksize, idx)
    // }

    // fn copy_file_range(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_in: PathBuf,
    //     file_handle_in: easy_fuser::prelude::BorrowedFileHandle,
    //     offset_in: i64,
    //     file_out: PathBuf,
    //     file_handle_out: easy_fuser::prelude::BorrowedFileHandle,
    //     offset_out: i64,
    //     len: u64,
    //     flags: u32, // Not implemented yet in standard
    // ) -> easy_fuser::prelude::FuseResult<u32> {
    //     self.get_inner().copy_file_range(
    //         req,
    //         file_in,
    //         file_handle_in,
    //         offset_in,
    //         file_out,
    //         file_handle_out,
    //         offset_out,
    //         len,
    //         easy_fuser::prelude::flags,
    //     )
    // }

    // Cant imagine a filesystem with no files.
    // Wait I actually can, the OS would just have to manipulate a LOT of directories...
    // Exercise for the reader, dir_fs where all files are just directories in disguise.
    // ^ If you actually make this, email me! - DocJade

    // easy_fuser docs say:
    // // If this method is not implemented or under Linux kernel versions earlier than
    // // 2.6.15, the mknod() and open() methods will be called instead.
    // Thus, we dont need this function.
    // fn create(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     parent_id: PathBuf,
    //     name: &std::ffi::OsStr,
    //     mode: u32,
    //     umask: u32,
    //     flags: easy_fuser::prelude::OpenFlags,
    // ) -> easy_fuser::prelude::FuseResult<(
    //     easy_fuser::prelude::OwnedFileHandle,
    //     <PathBuf as easy_fuser::prelude::FileIdType>::Metadata,
    //     easy_fuser::prelude::FUSEOpenResponseFlags,
    // )> {
    //
    // }

    // Unknown if this is needed. We'll find out.
    // fn fallocate(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    //     offset: i64,
    //     length: i64,
    //     mode: easy_fuser::prelude::FallocateFlags,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().fallocate(
    //         req,
    //         file_id,
    //         easy_fuser::prelude::file_handle,
    //         offset,
    //         length,
    //         mode,
    //     )
    // }

    // fn forget(&self, req: &easy_fuser::prelude::RequestInfo, file_id: PathBuf, nlookup: u64) {
    //     self.get_inner().forget(req, file_id, nlookup);
    // }

    // "This call is pretty much required for a usable filesystem." okay fine.
    fn getattr(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        _file_handle: Option<easy_fuser::prelude::BorrowedFileHandle>,
    ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::FileAttribute> {
        info!("Getting attributes of `{}`", file_id.display());

        // We only return things we actually know about the file, spoofing everything else.

        // This method is called on files and directories, including the root directory.

        // Chop off the head of the file id
        let item_name = match file_id.file_name() {
            Some(ok) => Some(ok),
            None => {
                // If there is no item name here, that means we are trying
                // to get information about the root directory.
                // We can skip finding the item lower down by just looking at inode 0 of root block.
                info!("This the root dir.");
                None
            }
        };

        // Yes i know that match statement is pointless, it just makes documenting it easier.

        // Get the directory path, but if this is `None`, we just mean the root directory
        let directory_path = match file_id.parent() {
            Some(ok) => ok,
            None => {
                // This must be in the root directory.
                Path::new("")
            }
        };

        let found_dir: DirectoryBlock;
        if let Some(directory) = DirectoryBlock::try_find_directory(directory_path)? {
            found_dir = directory
        } else {
            // No such directory!
            warn!("Parent directory of item we want attributes from does not exist!");
            warn!("Getting attributes failed!");
            return Err(NO_SUCH_ITEM.to_owned());
        };

        // Now get the file/dir

        // To find the item, we need to know if its a file or a directory.

        // We don't actually have to do a lookup if we are trying to find info on the root.
        let the_item: DirectoryItem;
        let is_file: bool = is_this_a_file(&file_id);

        if let Some(name) = item_name {
            // We are looking for something other than the root directory

            let name_stringified: String = name.to_str().expect("Should be valid utf8").to_string();
            let to_find: NamedItem = if is_file {
                // This is a file
                NamedItem::File(name_stringified)
            } else {
                // it must be a folder
                NamedItem::Directory(name_stringified)
            };
            // Go find it
            the_item = match found_dir.find_item(&to_find)? {
                Some(ok) => ok,
                None => {
                    // The item did not exist
                    warn!("Tried to get attributes of a file that did not exist!");
                    warn!("Getting attributes failed!");
                    return Err(NO_SUCH_ITEM.to_owned());
                }
            };
        } else {
            info!("Constructing root directory item...");
            // This is the root. We will spoof a directory item for it
            the_item = DirectoryItem {
                flags: DirectoryFlags::IsDirectory,
                name_length: 1,
                name: "/".to_string(),
                location: Pool::root_inode_location(),
            };
            info!("Done.");
        }

        // Now we need to get more info about it

        // The size of the item
        info!("Getting size of item...");
        let item_size: u64 = the_item.get_size()?;
        info!("Done.");

        // Creation and modification time. We do not support access time.
        info!("Getting timestamps...");
        let creation_time = the_item.get_crated_time()?;
        let modification_time = the_item.get_modified_time()?;
        info!("Done.");

        // Finally, the type of the file
        info!("Deducing item type...");
        let file_type: FileKind = if is_file {
            info!("It's a file.");
            FileKind::RegularFile
        } else {
            info!("It's a directory.");
            FileKind::Directory
        };
        info!("Done.");

        // Assemble all the info
        info!("Spoofing the attribute...");
        let attribute: FileAttribute =
            spoofed_file_attributes(item_size, creation_time, modification_time, file_type);
        info!("Done.");

        // All done!
        info!("Got attributes successfully.");
        Ok(attribute)
    }

    // Lock related, do we need this?
    // "If you want locking to work, you will need to implement the lock function"
    // To be frank, I dont really care. And i trust Frank so we wont do it.
    // // fn getlk(
    // //     &self,
    // //     req: &easy_fuser::prelude::RequestInfo,
    // //     file_id: PathBuf,
    // //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    // //     lock_owner: u64,
    // //     lock_info: easy_fuser::prelude::LockInfo,
    // // ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::LockInfo> {
    // //     self.get_inner().getlk(
    // //         req,
    // //         file_id,
    // //         easy_fuser::prelude::file_handle,
    // //         lock_owner,
    // //         lock_info,
    // //     )
    // // }

    // "This should be implemented only if HAVE_SETXATTR is true." Guess we ain't doin that.
    // fn getxattr(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     name: &std::ffi::OsStr,
    //     size: u32,
    // ) -> easy_fuser::prelude::FuseResult<Vec<u8>> {
    //     self.get_inner().getxattr(req, file_id, name, size)
    // }

    // No idea what this would even do
    // "Support the ioctl(2) system call. As such, almost everything is up to the filesystem."
    // Lets just imagine this call explodes the floppy drive, thus we will not implement it.
    // fn ioctl(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    //     flags: easy_fuser::prelude::IOCtlFlags,
    //     cmd: u32,
    //     in_data: Vec<u8>,
    //     out_size: u32,
    // ) -> easy_fuser::prelude::FuseResult<(i32, Vec<u8>)> {
    //     self.get_inner().ioctl(
    //         req,
    //         file_id,
    //         easy_fuser::prelude::file_handle,
    //         easy_fuser::prelude::flags,
    //         cmd,
    //         in_data,
    //         out_size,
    //     )
    // }

    // We wont allow links.
    // fn link(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     newparent: PathBuf,
    //     newname: &std::ffi::OsStr,
    // ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    // {
    //     self.get_inner().link(req, file_id, newparent, newname)
    // }

    // We wont have HAVE_SETXATTR
    // fn listxattr(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     size: u32,
    // ) -> easy_fuser::prelude::FuseResult<Vec<u8>> {
    //     self.get_inner().listxattr(req, file_id, size)
    // }

    // Not mentioned in the Geoff page, but this relates to `pathname lookup` so...
    // Gets info about a file in a directory
    fn lookup(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
    ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    {
        info!(
            "Looking up `{}` in `{}`",
            name.display(),
            parent_id.display()
        );
        // Since we use PathBuf, we dont need to return inode information, we can just call getattr() !
        // So just stick the file name back onto the parent and call.
        let joined: PathBuf = parent_id.join(name);

        // Go get em!
        // We dont use file handles
        info!("Getting attributes...");
        let result = self.getattr(req, joined, None)?;
        info!("Done.");

        // All done
        info!("Looked up file successfully.");
        Ok(result)
    }

    // Not mentioned, Assuming we're tracking where we are in files when handing out those BorrowedFileHandle's, in theory
    // we could do seeking. But if possible I would prefer to not let the OS hold onto any information to make my life easier in
    // case the OS's perspective gets de-synced... bhopping...
    // // fn lseek(
    // //     &self,
    // //     req: &easy_fuser::prelude::RequestInfo,
    // //     file_id: PathBuf,
    // //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    // //     seek: std::io::SeekFrom,
    // // ) -> easy_fuser::prelude::FuseResult<i64> {
    // //     self.get_inner()
    // //         .lseek(req, file_id, easy_fuser::prelude::file_handle, seek)
    // // }

    // Makes a directory
    // Yeah I think we need this.
    fn mkdir(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
        _mode: u32,
        _umask: u32,
    ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    {
        info!(
            "Making a new directory named `{}` at `{}`",
            name.display(),
            parent_id.display()
        );
        // We ignore the mode and umask. (related to file permissions)
        // All fluster directories are just epic like that.

        // Make sure the name of the folder isn't too long
        if name.len() > 255 {
            // Too long.
            warn!("Directory name is too long!");
            warn!("Failed to create directory!");
            return Err(FILE_NAME_TOO_LONG.to_owned());
        }

        // Open the folder
        let block: DirectoryBlock;

        if let Some(found) = DirectoryBlock::try_find_directory(&parent_id)? {
            // Directory does exist.
            block = found;
        } else {
            // No such directory.
            warn!("Parent directory does not exist!");
            warn!("Failed to create directory!");
            return Err(NO_SUCH_ITEM.to_owned());
        };

        // Make sure the directory we are trying to create does not already exist
        let new_name: String = name.to_str().expect("Should be valid utf8").to_string();
        if block
            .find_item(&NamedItem::Directory(new_name.clone()))?
            .is_some()
        {
            // A folder with that name already exists.
            warn!("Directory already exists!");
            warn!("Failed to create directory!");
            return Err(FILE_ALREADY_EXISTS.to_owned());
        }

        // Now that we have the directory, make the new directory
        info!("Creating directory...");
        block.make_directory(new_name)?;
        info!("Done.");

        // Now get info about the new directory.
        let new_location: PathBuf = parent_id.join(name);
        // We dont use file handles.
        info!("Getting attributes of the new directory...");
        let result = self.getattr(req, new_location)?;
        info!("Done.");

        // all done
        info!("Directory created successfully.");
        Ok(result)
    }

    // "This function is rarely needed, since it's uncommon to make these objects inside special-purpose filesystems."
    // Well, is fluster special-purpose? I guess? We'll see if this is needed, but for now, no.
    // fn mknod(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     parent_id: PathBuf,
    //     name: &std::ffi::OsStr,
    //     mode: u32,
    //     umask: u32,
    //     rdev: easy_fuser::prelude::DeviceType,
    // ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    // {
    //     self.get_inner()
    //         .mknod(req, parent_id, name, mode, umask, rdev)
    // }

    // "Open a file. If you aren't using file handles-" No geoff i am not
    // "-this function should just check for existence and permissions and return either success or an error code"
    // Seems reasonable. Whens the last time you used a file system that didn't support opening files?
    fn open(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        flags: easy_fuser::prelude::OpenFlags,
    ) -> easy_fuser::prelude::FuseResult<(
        easy_fuser::prelude::OwnedFileHandle,
        easy_fuser::prelude::FUSEOpenResponseFlags,
    )> {
        info!("Opening file `{}`", file_id.display());

        // There are some flags I don't quite understand, so we will go through them here
        if flags.contains(OpenFlags::MUST_BE_DIRECTORY) {
            // I think open should only be called on files, right?
            warn!("We can only open files, not directories!");
            warn!("Open failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }
        if flags.contains(OpenFlags::TEMPORARY_FILE) {
            // No temp files.
            warn!("We do not support temp files!");
            warn!("Open failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // Get the name and location of the file.
        let file_name: String = file_id
            .file_name()
            .expect("Files need a name to be created. Duh.")
            .to_str()
            .expect("Should be valid utf8")
            .to_string();
        let containing_folder = file_id.parent().expect("Files must go in folders.");

        // Make sure the file name isn't too long
        if file_name.len() > 255 {
            // Too long
            warn!("File name was too long!");
            warn!("Open failed!");
            return Err(FILE_NAME_TOO_LONG.to_owned());
        }

        // See if we are creating this file
        if flags.contains(OpenFlags::CREATE) {
            info!("We will be creating the file.");
            // We need to make this file first.
            // Open the directory
            if let Some(dir) = DirectoryBlock::try_find_directory(containing_folder)? {
                // Folder was real, make the file

                // Make sure file does not already exist.
                if dir
                    .find_item(&NamedItem::File(file_name.clone()))?
                    .is_some()
                {
                    info!("File already exists...");
                    // File already exists.

                    // Weirdly, we only need to fail here if a flag is set. Otherwise we just ignore this.
                    if flags.contains(OpenFlags::CREATE_EXCLUSIVE) {
                        warn!("...and thats's bad!");
                        warn!("Opening file failed!");
                        // We will fail.
                        return Err(FILE_ALREADY_EXISTS.to_owned());
                    }
                    info!("...but we don't care.");
                } else {
                    // But, we can still only create the file if it doesn't exist.
                    // We do not care about the resulting directory item, only if this fails.
                    info!("Creating file...");
                    let _ = dir.new_file(file_name.clone())?;
                    info!("Done.");
                    // File made. Continue!
                }
            } else {
                // Tried to make a file in a folder that did not exist
                warn!("The directory we tried to create the file in does not exist.!");
                warn!("Opening file failed!");
                return Err(NO_SUCH_ITEM.to_owned());
            };
            // File has been made.
        }

        // Now we actually read the file in again every time we do any kind of operation on them, so we just need to
        // check that the file exists before giving out a handle.

        // Go find the file

        // Open the containing directory
        if let Some(directory) = DirectoryBlock::try_find_directory(containing_folder)? {
            // Folder exists.
            if let Some(file) = directory.find_item(&NamedItem::File(file_name))? {
                // File exists, all good.

                // But we may need to truncate the file if asked.
                if flags.contains(OpenFlags::TRUNCATE) {
                    // Yep, truncate that sucker
                    info!("Truncation requested, truncating...");
                    file.truncate()?;
                    info!("Done.");
                }
            } else {
                // No such file.
                warn!("The the file was not present in the directory!");
                warn!("Opening file failed!");
                return Err(NO_SUCH_ITEM.to_owned());
            }
        } else {
            // No such folder.
            warn!("The directory we tried to open the file from does not exist.");
            warn!("Opening file failed!");
            return Err(NO_SUCH_ITEM.to_owned());
        }

        // Now, we need to return a file handle.
        // We don't actually use file handles though.
        let handle: OwnedFileHandle = love_handle();

        // We also need flags about opened files.

        let flags = open_response_flags();

        // All done!
        info!("File opened succesfully.");
        Ok((handle, flags))
    }

    // "Open a directory for reading." Thanks Geoff, I think I got that.
    // I mean, feels pretty fundamental... We'll do it I guess...

    // We actually don't do anything here, because this function is only used to
    // get file handles for directories. We dont support file handles.

    // If it's needed, I'll implement it, but looking at easy_fuser examples, it does not seem to be
    // mandatory.

    // fn opendir(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     flags: easy_fuser::prelude::OpenFlags,
    // ) -> easy_fuser::prelude::FuseResult<(
    //     easy_fuser::prelude::OwnedFileHandle,
    //     easy_fuser::prelude::FUSEOpenResponseFlags,
    // )> {

    //     // Make sure the directory exists
    //     if let Some(exist) = DirectoryBlock::try_find_directory(file_id)? {
    //         // It's there.
    //     } else {
    //         // it's not there. We do not create directories here.
    //         return Err(NO_SUCH_ITEM);
    //     };
    //
    //
    //     todo!();
    //     // self.get_inner()
    //     //     .opendir(req, file_id, easy_fuser::prelude::flags)
    // }

    // "Read `size` bytes from the given file into the buffer `buf`, beginning offset bytes into the file."
    // "Required for any sensible filesystem." Yeah I bet.
    // It looks like we don't have some async buffer to write into, so we'll have to write the whole thing.
    // Although I did see whispers of a max read size setting... if we set that small-ish, then there shouldn't
    // be a massive latency problem from trying to load 30MB files or something.
    // What's a reasonable max though? 1MB? We'll see.
    fn read(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        _file_handle: easy_fuser::prelude::BorrowedFileHandle,
        seek: std::io::SeekFrom,
        size: u32,
        _flags: easy_fuser::prelude::FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> easy_fuser::prelude::FuseResult<Vec<u8>> {
        info!(
            "Attempting to read `{}` bytes out of file `{}`",
            size,
            file_id.display()
        );

        // We only support seeking from the start of files, since we do not store seek information.
        // Will this be an issue? We'll find out later.
        let byte_offset: u64 = match seek {
            std::io::SeekFrom::Start(bytes) => bytes,
            _ => {
                warn!("Tried to seek from non-start point. We do not support this!");
                warn!("Read failed!");
                // Only support start seeks.
                return Err(ILLEGAL_SEEK.to_owned());
            }
        };
        info!(
            "We will be reading the file at a seek offset of `{}`.",
            byte_offset
        );

        // Open the file if it exists.
        if let Some(dir) =
            DirectoryBlock::try_find_directory(file_id.parent().expect("Files live in folders."))?
        {
            // directory exists.
            // Open the file
            let finder: NamedItem = NamedItem::File(
                file_id
                    .file_name()
                    .expect("Files have names.")
                    .to_str()
                    .expect("Should be valid utf8")
                    .to_string(),
            );

            if let Some(found_file) = dir.find_item(&finder)? {
                // File exists
                // Read it
                // `size` is how many bytes are attempting to be read.
                info!("File exists, reading from it...");
                let read_result = found_file.read_file(byte_offset, size)?;
                info!("Done.");

                // All done!
                info!("File read successfully.");
                Ok(read_result)
            } else {
                // No such file.
                warn!("There is no file to read from!");
                warn!("Read failed!");
                Err(NO_SUCH_ITEM.to_owned())
            }
        } else {
            // No such directory.
            warn!("The parent directory of the file we wanted to read did not exist!");
            warn!("Read failed!");
            Err(NO_SUCH_ITEM.to_owned())
        }
        // Unreachable down here.
    }

    // "Return one or more directory entries (struct dirent) to the caller."
    // "This is one of the most complex FUSE functions." Oof.
    // "The readdir function is somewhat like read, in that it starts at a
    //  given offset and returns results in a caller-supplied buffer."
    // "However, the offset not a byte offset" What the hell
    // "...and the results are a series of struct dirents rather than being uninterpreted bytes" those are just words Geoffery
    // It seems that easy_fuser at least has a special type for it, a Vec of tuple of string and file metadata... Might not be
    // too bad, almost seems like it could be deconstructed into other calls... Might do that.

    fn readdir(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        _file_handle: easy_fuser::prelude::BorrowedFileHandle,
    ) -> easy_fuser::prelude::FuseResult<
        Vec<(
            std::ffi::OsString,
            <PathBuf as easy_fuser::prelude::FileIdType>::MinimalMetadata,
        )>,
    > {
        info!("Getting contents of directory `{}`", file_id.display());

        // This seems to just be a list call.

        // The metadata only needs to be the type of the file according to easy_fuser docs:
        // /// For PathBuf-based: FileKind
        // /// - User only needs to provide FileKind; Inode is managed internally.

        // I'm assuming the incoming path is a directory, not a file.
        if is_this_a_file(&file_id) {
            warn!("Tried to read a file as a directory!");
            warn!("Getting contents failed!");
            // Why are you reading a file as a directory
            return Err(NOT_A_DIRECTORY.to_owned());
        };

        // Load in the directory
        let requested_dir: DirectoryBlock;
        if let Some(exists) = DirectoryBlock::try_find_directory(&file_id)? {
            requested_dir = exists
        } else {
            // No such directory
            warn!("The directory does not exist!");
            warn!("Getting contents failed!");
            return Err(NO_SUCH_ITEM.to_owned());
        }

        // List the directory
        info!("Listing items...");
        let items = requested_dir.list()?;
        info!("Done.");

        // Now we need to construct the minimal metadata, which is easy since we only
        // need the file type.

        // (name, filetype)
        let mut output: Vec<(OsString, FileKind)> = Vec::new();

        info!("Extracting required file metadata...");
        for item in items {
            let name: OsString = item.name.into();
            let item_type: FileKind = if item.flags.contains(DirectoryFlags::IsDirectory) {
                // dir
                FileKind::Directory
            } else {
                // file
                FileKind::RegularFile
            };
            output.push((name, item_type));
        }
        info!("Done.");
        info!("Directory contents retrieved successfully.");

        // All done.
        Ok(output)
    }

    // is this HAVE_SETXATTR related? No idea.
    // For now, its a no from me. *dramatic TV stinger sound*
    // fn readdirplus(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    // ) -> easy_fuser::prelude::FuseResult<
    //     Vec<(
    //         std::ffi::OsString,
    //         <PathBuf as easy_fuser::prelude::FileIdType>::Metadata,
    //     )>,
    // > {
    //     let readdir_result =
    //         self.readdir(req, file_id.clone(), easy_fuser::prelude::file_handle)?;
    //     let mut result = Vec::with_capacity(readdir_result.len());
    //     for (name, _) in readdir_result.into_iter() {
    //         let metadata = self.lookup(req, file_id.clone(), &name)?;
    //         result.push((name, metadata));
    //     }
    //     Ok(result)
    // }

    // No links
    // fn readlink(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    // ) -> easy_fuser::prelude::FuseResult<Vec<u8>> {
    //     self.get_inner().readlink(req, file_id)
    // }

    // "Release is called when FUSE is completely done with a file;
    //  at that point, you can free up any temporarily allocated data structures."
    // Well in that case, we shouldn't have any temporary stuff. So this function is free.
    fn release(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        _file_id: PathBuf,
        _file_handle: easy_fuser::prelude::OwnedFileHandle,
        _flags: easy_fuser::prelude::OpenFlags,
        _lock_owner: Option<u64>,
        _flush: bool,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // self.get_inner().release(
        //     req,
        //     file_id,
        //     easy_fuser::prelude::file_handle,
        //     easy_fuser::prelude::flags,
        //     lock_owner,
        //     flush,
        // )
        Ok(())
    }

    // Same as release
    fn releasedir(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        _file_id: PathBuf,
        _file_handle: easy_fuser::prelude::OwnedFileHandle,
        _flags: easy_fuser::prelude::OpenFlags,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // self.get_inner().releasedir(
        //     req,
        //     file_id,
        //     easy_fuser::prelude::file_handle,
        //     easy_fuser::prelude::flags,
        // )
        Ok(())
    }

    // HAVE_SETXATTR? More like HAVE_SEXATTR
    // fn removexattr(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     name: &std::ffi::OsStr,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().removexattr(req, file_id, name)
    // }

    // No renaming, yeah it wouldnt be impossible to implement but we shouldn't need that for Factorio.
    // fn rename(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     parent_id: PathBuf,
    //     name: &std::ffi::OsStr,
    //     newparent: PathBuf,
    //     newname: &std::ffi::OsStr,
    //     flags: easy_fuser::prelude::RenameFlags,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().rename(
    //         req,
    //         parent_id,
    //         name,
    //         newparent,
    //         newname,
    //         easy_fuser::prelude::flags,
    //     )
    // }

    // "Remove the given directory.
    // This should succeed only if the directory is empty (except for "." and "..")."
    // Seems easy enough.
    fn rmdir(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
    ) -> easy_fuser::prelude::FuseResult<()> {
        info!(
            "Attempting to remove directory `{}` contained within `{}`",
            name.display(),
            parent_id.display()
        );
        // Open the directory
        let parent: DirectoryBlock;
        if let Some(exists) = DirectoryBlock::try_find_directory(&parent_id)? {
            // parent is there
            parent = exists;
        } else {
            // Cant remove a directory from a non-existant parent directory.
            warn!("Parent directory did not exist!");
            warn!("Directory removal failed!");
            return Err(NO_SUCH_ITEM.to_owned());
        }

        // Does the directory we want to remove exist?
        let stringed_name: String = name.to_str().expect("Should be vaid utf8").to_string();
        let extracted: DirectoryItem;
        if let Some(found) = parent.extract_item(&NamedItem::Directory(stringed_name))? {
            // Directory exists, and we have extracted it from the parent.
            extracted = found;
        } else {
            // Cant delete a dir that isnt there
            warn!("There isn't a directory with that name in the parent folder.");
            warn!("Directory removal failed!");
            return Err(NO_SUCH_ITEM.to_owned());
        }

        // Get the directory from the item we extracted
        let to_delete_pointer: DiskPointer = extracted
            .get_inode()?
            .extract_directory()
            .expect("This should be a directory.")
            .pointer;

        // Open the block
        let to_delete_dir: DirectoryBlock = DirectoryBlock::from_block(&CachedBlockIO::read_block(
            to_delete_pointer,
            JustDiskType::Standard,
        )?);

        // Go delete it.
        info!("Removing directory...");
        to_delete_dir.delete_directory()?;
        info!("Done.");

        // All done.
        info!("Directory removed.");
        Ok(())
    }

    // This HAS to be mandatory right?
    // Linux > Yo what kinda file is this
    // Fluster! > Boi if you don't shut yo flightless bird ass up ima whoop yo ass
    // *Linux has SIGKILL'ed Fluster!*
    fn setattr(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        attrs: easy_fuser::prelude::SetAttrRequest,
    ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::FileAttribute> {
        info!(
            "Attemping to set attributes on file `{}`",
            file_id.display()
        );

        // We can ignore almost all of this

        // From easy_fuser:

        // pub struct SetAttrRequest<'a> {
        //     /// File mode (permissions)
        //     pub mode: Option<u32>,
        //     /// User ID of the file owner
        //     pub uid: Option<u32>,
        //     /// Group ID of the file owner
        //     pub gid: Option<u32>,
        //     /// File size in bytes
        //     pub size: Option<u64>,
        //     /// Last access time
        //     pub atime: Option<TimeOrNow>,
        //     /// Last modification time
        //     pub mtime: Option<TimeOrNow>,
        //     /// Last status change time
        //     pub ctime: Option<SystemTime>,
        //     /// Creation time
        //     pub crtime: Option<SystemTime>,
        //     /// Change time (for BSD systems)
        //     pub chgtime: Option<SystemTime>,
        //     /// Backup time (for macOS)
        //     pub bkuptime: Option<SystemTime>,
        //     /// File flags (unused in FUSE)
        //     pub flags: Option<()>,
        //     /// File handle for the file being modified
        //     pub file_handle: Option<BorrowedFileHandle<'a>>,
        // }

        // Manually listing these out in case i end up needing to support some of these.

        // File permissions cannot be changed.
        if attrs.mode.is_some() {
            warn!("Changing permissions is not supported!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // File owner cannot be changed.
        if attrs.uid.is_some() {
            warn!("Changing owner is not supported!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // File owner group cannot be changed.
        if attrs.gid.is_some() {
            warn!("Changing owner group is not supported!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // File size cannot be changed here. Making a file bigger is done by writing, and
        //  truncation is the only way to make a file smaller.
        if attrs.size.is_some() {
            warn!("Changing file size directly is not supported!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // File update times are handled lower down. We will not change them.
        if attrs.atime.is_some()
            || attrs.bkuptime.is_some()
            || attrs.chgtime.is_some()
            || attrs.crtime.is_some()
            || attrs.ctime.is_some()
            || attrs.mtime.is_some()
        {
            warn!("Changing time information is not supported!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // Flags are unused.
        if attrs.flags.is_some() {
            // What?
            unreachable!()
        }

        // We dont use file handles
        if attrs.file_handle.is_some() {
            warn!("We dont use file handles!");
            warn!("Setting attributes failed!");
            return Err(NOT_SUPPORTED.to_owned());
        }

        // We just return what already exists
        info!("We will return the attributes that already exist...");
        let result = self.getattr(req, file_id, None)?;
        info!("Done");
        Ok(result)
    }

    // No file locking.
    // fn setlk(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     file_handle: easy_fuser::prelude::BorrowedFileHandle,
    //     lock_owner: u64,
    //     lock_info: easy_fuser::prelude::LockInfo,
    //     sleep: bool,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().setlk(
    //         req,
    //         file_id,
    //         easy_fuser::prelude::file_handle,
    //         lock_owner,
    //         lock_info,
    //         sleep,
    //     )
    // }

    // Again with these extended mfs
    // fn setxattr(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     name: &std::ffi::OsStr,
    //     value: Vec<u8>,
    //     flags: easy_fuser::prelude::FUSESetXAttrFlags,
    //     position: u32,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().setxattr(
    //         req,
    //         file_id,
    //         name,
    //         value,
    //         easy_fuser::prelude::flags,
    //         position,
    //     )
    // }

    // Who would even call this?
    // "Not required, but handy for read/write filesystems since this is how programs like df determine the free space."
    // I mean, if there's an issue were creating files fails due to it not seeing enough space, maybe.
    // fn statfs(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    // ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::StatFs> {
    //     self.get_inner().statfs(req, file_id)
    // }

    // No links
    // fn symlink(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     parent_id: PathBuf,
    //     link_name: &std::ffi::OsStr,
    //     target: &std::path::Path,
    // ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    // {
    //     self.get_inner().symlink(req, parent_id, link_name, target)
    // }

    // Returns how many bytes were written
    // We could grab pool statistics from here...
    fn write(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        _file_handle: easy_fuser::prelude::BorrowedFileHandle,
        seek: std::io::SeekFrom,
        data: Vec<u8>,
        _write_flags: easy_fuser::prelude::FUSEWriteFlags,
        _flags: easy_fuser::prelude::OpenFlags,
        _lock_owner: Option<u64>,
    ) -> easy_fuser::prelude::FuseResult<u32> {
        info!(
            "Attempting to write `{}` bytes to file `{}`",
            data.len(),
            file_id.display()
        );

        // Get the file
        // We only support seeking from the start of files, since we do not store seek information.
        // Will this be an issue? We'll find out later.
        let byte_offset: u64 = match seek {
            std::io::SeekFrom::Start(bytes) => bytes,
            _ => {
                // Only support start seeks.
                warn!("Cannot seek from anywhere but the start of the file!");
                return Err(ILLEGAL_SEEK.to_owned());
            }
        };

        // Open the file if it exists.
        if let Some(dir) =
            DirectoryBlock::try_find_directory(file_id.parent().expect("Files live in folders."))?
        {
            // directory exists.
            // Open the file
            let finder: NamedItem = NamedItem::File(
                file_id
                    .file_name()
                    .expect("Files have names.")
                    .to_str()
                    .expect("Should be valid utf8")
                    .to_string(),
            );

            if let Some(found_file) = dir.find_item(&finder)? {
                // File exists
                // Write to it.
                info!("Writing data...");
                let bytes_written: u32 = found_file.write_file(&data, byte_offset)?;
                info!("Done.");

                // All done!
                info!("Write successful!");
                Ok(bytes_written)
            } else {
                // No such file.
                warn!("The file we are trying to write do does not exist!");
                warn!("Writing to file failed!");
                Err(NO_SUCH_ITEM.to_owned())
            }
        } else {
            // No such directory.
            warn!("Parent folder does not exist!");
            warn!("Writing to file failed!");
            Err(NO_SUCH_ITEM.to_owned())
        }
    }

    // Not sure if this is file deletion or symlink related.
    // I'll need to investigate.
    // fn unlink(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     parent_id: PathBuf,
    //     name: &std::ffi::OsStr,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().unlink(req, parent_id, name)
    // }
}
