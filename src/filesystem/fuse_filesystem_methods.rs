// The actual FUSE filesystem layer.

//
//
// ======
// Imports
// ======
//
//

use std::{path::Path, time::Duration};

use fuse_mt::{FileAttr, FilesystemMT};
use log::{debug, info, warn};

use crate::{
    filesystem::{filesystem_struct::FlusterFS, item_flag::flag_struct::ItemFlag},
    pool::disk::{generic::io::cache::cache_io::CachedBlockIO, standard_disk::block::{directory::directory_struct::{DirectoryBlock, DirectoryFlags, DirectoryItem}, io::directory::types::NamedItem}}
};

use super::file_handle::file_handle_struct::FileHandle;
use super::error::error_types::*;

use fuse_mt::CreatedEntry;




//
//
// ======
// Helper functions
// ======
//
//

//
//
// ======
// The actual fuse layer
// ======
//
// There's a lot of stuff in here we technically dont need. And I'm going to assume the information on this page is correct
// https://www.cs.hmc.edu/~geoff/classes/hmc.cs135.201001/homework/fuse/fuse_doc.html
// I have archived this page on internet archive.
// Thanks Geoff! I hope your life is going well, 16 years later.
//
// In theory, some calls like truncate() should be handled by the os before other operations here. We will still check for those flags, just in case.
//
// Also, in theory again, we should NEVER modify the flags before returning them. Linux VFS depends on this (in theory)

impl FilesystemMT for FlusterFS {
    // The most British function in Fluster
    fn init(&self, _req: fuse_mt::RequestInfo) -> fuse_mt::ResultEmpty {
        // In the old crate, we could do stuff like set the max write size to the filesystem here.
        // We need to figure out how to do that elsewhere.
        // TODO: Set max write size to one megabyte, and disable kernel lookahead if possible.
        Ok(())
    }

    // Called when filesystem is unmounted. Should flush all data to disk.
    fn destroy(&self) {
        info!("Shutting down filesystem...");
        // Flush all of the tiers of cache.
        info!("Flushing cache...");
        CachedBlockIO::flush().expect("I sure hope flushing works.");
        // Now flush pool information
        todo!("Flush pool info to pool disk.");
        info!("Done.");
        info!("Goodbye! .o/");
    }

    // Get file attributes of an item.
    fn getattr(
        &self,
        _req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        fh: Option<u64>,
    ) -> fuse_mt::ResultEntry {
        debug!("Getting attributes of `{}`...", path.display());
        // This wants a TTL again, ok...
        // TODO: Add a ttl setting to FlusterFS type
        let a_year: Duration = Duration::from_secs(60*60*24*365);

        // I already wrote a method for this yay
        // but that assumes we have a handle.
        if let Some(handle) = fh {
            debug!("Handle was provided, getting and returning attributes.");
            // Handle exists, easy path.
            return Ok(
                (
                    a_year,
                    FileHandle::read(handle).try_into()?
                )
            )
        }

        // No handle, dang.
        
        // Go find that sucker

        // Making a temporary handle (doesn't need to be allocated) lets us call some easier methods.
        // We cant just use the TryInto FileAttr since we dont know for sure if the item exists yet.
        let temp_handle: FileHandle = FileHandle {
            path: path.into(),
            flags: ItemFlag::empty(),
        };

        // Get the item type
        let item_to_find: NamedItem = temp_handle.get_named_item();

        let found_item: DirectoryItem;

        // Directory
        debug!("Searching for item...");
        if let Some(parent) = DirectoryBlock::try_find_directory(path.parent())? {
            // Item
            if let Some(item) = parent.find_item(&item_to_find)? {
                debug!("Item found.");
                found_item = item;
            } else {
                // item did not exist.
                debug!("Item was not present in parent.");
                return Err(NO_SUCH_ITEM)
            }
        } else {
            // Parent does not exist. We cannot get attributes.
            debug!("Parent directory did not exist for this item.");
            return Err(NO_SUCH_ITEM)
        }

        // Get the attributes
        debug!("Getting attributes of item...");
        let found_attributes: FileAttr = found_item.try_into()?;
        debug!("Done! Returning.");

        return Ok(
            (
                a_year,
                found_attributes
            )
        );
    }

    // We dont support file permissions.
    
    // fn chmod(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _fh: Option<u64>,
    //     _mode: u32,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // We dont support file permissions.
    // fn chown(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _fh: Option<u64>,
    //     _uid: Option<u32>,
    //     _gid: Option<u32>,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // File truncation is supported.
    // Does not always truncate file to 0 bytes long.
    fn truncate(
        &self,
        _req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        fh: Option<u64>,
        size: u64,
    ) -> fuse_mt::ResultEmpty {
        debug!("Truncating `{}` to be `{}` bytes long...", path.display(), size);
        // Get a file handle
        let handle: FileHandle = if let Some(exists) = fh {
            debug!("File handle was passed in, using that...");
            // Got a handle from the call, no fancy work.
            // Read it in
            FileHandle::read(exists)
        } else {
            debug!("No handle provided, spoofing...");
            // Temp handle that we will not allocate.
            FileHandle {
                path: path.into(),
                flags: ItemFlag::empty(),
            }
        };

        debug!("Handle obtained.");

        // You cannot truncate directories.
        if !handle.is_file() {
            warn!("Attempted to truncate a directory. Ignoring.");
            return Err(IS_A_DIRECTORY)
        }

        // Go load the file to truncate
        let item_to_find: NamedItem = handle.get_named_item();
        let found_item: DirectoryItem;

        debug!("Searching for item...");
        if let Some(parent) = DirectoryBlock::try_find_directory(path.parent())? {
            // Item
            if let Some(item) = parent.find_item(&item_to_find)? {
                debug!("Item found.");
                found_item = item;
            } else {
                // item did not exist.
                debug!("Item was not present in parent.");
                return Err(NO_SUCH_ITEM)
            }
        } else {
            // Parent does not exist. We cannot get attributes.
            debug!("Parent directory did not exist for this item.");
            return Err(NO_SUCH_ITEM)
        }

        // Now with the directory item, we can run the truncation.
        debug!("Starting truncation...");
        found_item.truncate(size)?;
        debug!("Truncation finished.");
        // All done.
        Ok(())
    }

    // We do not support manually updating timestamps.
    // fn utimens(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _fh: Option<u64>,
    //     _atime: Option<std::time::SystemTime>,
    //     _mtime: Option<std::time::SystemTime>,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // We do not support manually updating timestamps.
    // fn utimens_macos(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _fh: Option<u64>,
    //     _crtime: Option<std::time::SystemTime>,
    //     _chgtime: Option<std::time::SystemTime>,
    //     _bkuptime: Option<std::time::SystemTime>,
    //     _flags: Option<u32>,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // We do not support symbolic links.
    // fn readlink(&self, _req: fuse_mt::RequestInfo, _path: &std::path::Path) -> fuse_mt::ResultData {
    //     Err(libc::ENOSYS)
    // }

    // "This function is rarely needed, since it's uncommon to make these objects inside special-purpose filesystems."
    // This is for fancy things like block objects i believe, we do not support this.
    // fn mknod(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _parent: &std::path::Path,
    //     _name: &std::ffi::OsStr,
    //     _mode: u32,
    //     _rdev: u32,
    // ) -> fuse_mt::ResultEntry {
    //     Err(libc::ENOSYS)
    // }

    // Create a new directory if it does not already exist.
    // Returns file attributes about the new directory
    fn mkdir(
        &self,
        _req: fuse_mt::RequestInfo,
        parent: &std::path::Path,
        name: &std::ffi::OsStr,
        _mode: u32, // Permission bit related. Do not need.
    ) -> fuse_mt::ResultEntry {
        debug!("Creating new directory in `{}` named `{}`.", parent.display(), name.display());
        // Make sure the name isn't too long
        if name.len() > 255 {
            debug!("Name is too long.");
            return Err(FILE_NAME_TOO_LONG);
        }

        // the new directory
        let new_dir: DirectoryItem;
        let the_name: String = name.to_str().expect("Should be valid utf8").to_string()

        // Open parent
        if let Some(parent) = DirectoryBlock::try_find_directory(Some(parent))? {
            debug!("Checking if directory exists...");
            if parent.find_item(&NamedItem::Directory(the_name.clone()))?.is_some() {
                // Directory already exists.
                debug!("Directory already exists.");
                return Err(ITEM_ALREADY_EXISTS)
            }
            
            // Make the directory
            debug!("It did not, creating directory...");
            new_dir = parent.make_directory(the_name)?;
            debug!("Directory created.");
        } else {
            // No such parent
            debug!("Parent did not exist.");
            return Err(NO_SUCH_ITEM);
        }

        // Now we need attribute information about it.
        debug!("Getting attribute info...");
        let attributes: FileAttr = new_dir.try_into()?;
        debug!("Done.");

        let a_year: Duration = Duration::from_secs(60*60*24*365);

        // All done!
        debug!("Directory created successfully.");
        Ok(
            (
                a_year,
                attributes
            )
        )
    }

    // Deletes a file.
    fn unlink(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(UNIMPLEMENTED)
    }

    // Deletes a directory.
    // Should fail if the directory is not empty.
    fn rmdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(UNIMPLEMENTED)
    }

    // We do not support symlinks.
    // fn symlink(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _parent: &std::path::Path,
    //     _name: &std::ffi::OsStr,
    //     _target: &std::path::Path,
    // ) -> fuse_mt::ResultEntry {
    //     Err(libc::ENOSYS)
    // }

    // Renames / moves files.
    fn rename(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _newparent: &std::path::Path,
        _newname: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(UNIMPLEMENTED)
    }

    // We do not support hard links.
    // fn link(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _newparent: &std::path::Path,
    //     _newname: &std::ffi::OsStr,
    // ) -> fuse_mt::ResultEntry {
    //     Err(libc::ENOSYS)
    // }

    // Open a file and get a handle that will be used to access it.
    // Does not create files.
    fn open(
        &self,
        _req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        flags: u32,
    ) -> fuse_mt::ResultOpen {
        debug!("Opening item at path `{}`...", path.display());
        // Deduce the open permissions.
        debug!("Deducing flags...");
        let converted_flag: ItemFlag = ItemFlag::deduce_flag(flags)?;
        debug!("Ok.");

        // We require at least one of the read/write flags.
        // ...or, more correctly: we would require them if we used them.
        // We dont. Everything is read/write.

        // We ignore any flags that are not valid for this method, such as
        // truncation or creation flags.

        // open() always returns a brand new file handle, regardless if that file was
        // already open somewhere else.
        let handle: FileHandle = FileHandle {
            path: path.into(),
            flags: converted_flag,
        };

        // We do not allocate the file handle until we are sure we will use it.

        // Make sure the name of the file is not too long.
        if handle.name().len() > 255 {
            warn!("File name is too long.");
            // File name was too long.
            return Err(FILE_NAME_TOO_LONG)
        }

        // Load in info about where the file should be.
        // This will bail if a low level floppy issue happens.
        debug!("Attempting to load in the parent directory...");
        let containing_dir_block: DirectoryBlock = match DirectoryBlock::try_find_directory(handle.path.parent())? {
            Some(ok) => ok,
            None => {
                // Cannot load files from directories that do not exist.
                warn!("Directory that the item was supposed to be contained within does not exist.");
                return Err(NO_SUCH_ITEM)
            },
        };
        debug!("Directory loaded.");
        
        // At this point. We need to know if we are looking for a directory or a file.
        debug!("Deducing request item type...");
        let extracted_name = handle.name();
        let item_to_find: NamedItem = if handle.is_file() {
            // File
            debug!("Looking for a file...");
            // Cool beans.
            debug!("Named `{extracted_name}`.");
            NamedItem::File(extracted_name.to_string())
        } else {
            // Directory
            debug!("Looking for a directory...");
            debug!("Named `{extracted_name}`.");
            NamedItem::Directory(extracted_name.to_string())
        };

        // Hold onto the item until we need it
        let found_item: DirectoryItem;

        // Now load in the directory item.
        debug!("Attempting to find the item...");
        if let Some(exists) = containing_dir_block.find_item(&item_to_find)? {
            debug!("Item exists.");
            found_item = exists;
        } else {
            // No item
            debug!("Item does not exist.");
            return Err(NO_SUCH_ITEM);
        }

        // We have now loaded in the directory item, or bailed out if needed.

        // Assert that this is a directory if required.
        // In theory we could check this earlier, but it's good to ensure that the underlying
        // item agrees.
        if converted_flag.contains(ItemFlag::ASSERT_DIRECTORY) {
            debug!("Caller wants to ensure they are opening a directory.");
            if !found_item.flags.contains(DirectoryFlags::IsDirectory) {
                debug!("This is not a directory.");
                return Err(NOT_A_DIRECTORY)
            }
            debug!("This is a directory.");
        }

        // We are done creating/loading the file, its time to get a handle.
        let new_handle: u64 = handle.allocate();

        // Done!
        Ok((new_handle, converted_flag.into()))
    }

    // Read file data from a file handle.
    // "Note that it is not an error for this call to request to read past the end of the file,
    //   and you should only return data up to the end of the file
    //   (i.e. the number of bytes returned will be fewer than requested; possibly even zero).
    //   Do not extend the file in this case."
    //
    // Uses callbacks, wacky, not sure how that works.
    fn read(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _offset: u64,
        _size: u32,
        callback: impl FnOnce(fuse_mt::ResultSlice<'_>) -> fuse_mt::CallbackResult,
    ) -> fuse_mt::CallbackResult {
        callback(Err(UNIMPLEMENTED))
    }

    // Write data to a file using a file handle.
    fn write(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _offset: u64,
        _data: Vec<u8>,
        _flags: u32,
    ) -> fuse_mt::ResultWrite {
        Err(UNIMPLEMENTED)
    }

    // Flushing does not do anything, since we manually handle our caching.
    fn flush(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _lock_owner: u64,
    ) -> fuse_mt::ResultEmpty {

        // We dont want the OS to be able to flush the cache to disk, this could happen randomly for no reason.
        // We are responsible for tracking how stale the cache is.

        // The only time we will flush the cache from this level is on shutdown.
        
        Ok(())
    }

    // Releasing a file handle. Not sure if we need to do anything special with the passed in handle yet. We'll see.
    // Releasing things should have no effect, since we handle all of our own caching stuff, and don't support locks.
    fn release(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> fuse_mt::ResultEmpty {
        Err(UNIMPLEMENTED)
    }

    // See flush()
    fn fsync(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _datasync: bool,
    ) -> fuse_mt::ResultEmpty {
        Ok(())
    }

    // Open a directory and get a handle to it
    fn opendir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _flags: u32,
    ) -> fuse_mt::ResultOpen {
        Err(UNIMPLEMENTED)
    }

    // List the contents of a directory.
    fn readdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
    ) -> fuse_mt::ResultReaddir {
        Err(UNIMPLEMENTED)
    }

    // See release()
    fn releasedir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _flags: u32,
    ) -> fuse_mt::ResultEmpty {
        Ok(())
    }

    // See flush()
    fn fsyncdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _datasync: bool,
    ) -> fuse_mt::ResultEmpty {
        Ok(())
    }

    // Get file system statistics.
    // Seemingly contains information about the file system, like optimal block size and max file name length
    fn statfs(&self, _req: fuse_mt::RequestInfo, _path: &std::path::Path) -> fuse_mt::ResultStatfs {
        // TODO: Is this needed?
        Err(UNIMPLEMENTED)
    }

    // Extended attributes are not supported.
    // fn setxattr(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _name: &std::ffi::OsStr,
    //     _value: &[u8],
    //     _flags: u32,
    //     _position: u32,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // Extended attributes are not supported.
    // fn getxattr(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _name: &std::ffi::OsStr,
    //     _size: u32,
    // ) -> fuse_mt::ResultXattr {
    //     Err(libc::ENOSYS)
    // }

    // Extended attributes are not supported.
    // fn listxattr(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _size: u32,
    // ) -> fuse_mt::ResultXattr {
    //     Err(libc::ENOSYS)
    // }

    // Extended attributes are not supported.
    // fn removexattr(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _name: &std::ffi::OsStr,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // "This call is not required but is highly recommended." Okay then we wont do it muhahaha
    // fn access(
    //     &self,
    //     _req: fuse_mt::RequestInfo,
    //     _path: &std::path::Path,
    //     _mask: u32,
    // ) -> fuse_mt::ResultEmpty {
    //     Err(libc::ENOSYS)
    // }

    // Creates and opens a new file, returns a file handle.
    fn create(
        &self,
        req: fuse_mt::RequestInfo,
        parent: &std::path::Path,
        name: &std::ffi::OsStr,
        _mode: u32,
        flags: u32,
    ) -> fuse_mt::ResultCreate {
        debug!("Creating new file named `{}` in `{}`...", name.display(), parent.display());

        // Extract the flags
        // Will bail if needed.
        let deduced_flags: ItemFlag = ItemFlag::deduce_flag(flags)?;

        // Is the name too long?
        if name.len() > 255 {
            debug!("File name is too long. Bailing.");
            return Err(FILE_NAME_TOO_LONG)
        }

        // Try and load in the parent directory
        // This will bail if a low level floppy issue happens.
        debug!("Attempting to load in the parent directory...");
        let containing_dir_block: DirectoryBlock = match DirectoryBlock::try_find_directory(Some(parent))? {
            Some(ok) => ok,
            None => {
                // Nope, no parent.
                warn!("Cannot create files in directories that do not exist.");
                return Err(NO_SUCH_ITEM)
            },
        };
        debug!("Directory loaded.");
        
        // Make sure the file does not already exist.
        debug!("Checking if file already exists...");
        let converted_name: String = name.to_str().expect("Should be valid UTF8.").to_string();
        // Will bail if needed.
        if let Some(_exists) = containing_dir_block.find_item(&NamedItem::File(converted_name.clone()))? {
            debug!("File already exists.");
            // But do we care?
            if deduced_flags.contains(ItemFlag::CREATE_EXCLUSIVE) {
                // Yes we do, this is a failure.
                debug!("Caller wanted to create this file, not open it. Bailing.");
                return Err(ITEM_ALREADY_EXISTS)
            }
            
            // Since the file already exists we can skip the creation process.
            // just load it in as usual.
            
            // Full item path
            let constructed_path: &Path = &parent.join(name);
            
            // Dont care about the returned flags, they wont change anyways.
            let (file_handle, _): (u64, u32) = self.open(req, constructed_path, flags)?;
            
            // Get the innards of the handle
            let handle_inner: FileHandle = FileHandle::read(file_handle);
            
            // Get the metadata from that
            debug!("Getting file attributes...");
            let facebook_data: FileAttr = handle_inner.try_into()?;
            
            // Put it all together
            // No idea what the TTL should be set to. I'm assuming that's how long the handles last?
            // I will never drop handles on my side, the OS has to drop em.
            debug!("Done reading in file, returning.");
            return Ok(CreatedEntry {
                ttl: Duration::from_secs(60*60*24*365), // A year sounds good.
                attr: facebook_data,
                fh: file_handle,
                flags,  // We use the same flags we came in with. Not the one from the loaded file.
                        // Is that a bad idea? No idea. TODO: is this safe?
            })
        }
        
        // File did not exist, actually creating it...
        debug!("Creating file...");
        let resulting_item: DirectoryItem = containing_dir_block.new_file(converted_name)?;
        debug!("Creating file...");

        // Full item path
        let constructed_path: &Path = &parent.join(name);

        // Construct and return the handle to the new file
        let new_handle: FileHandle = FileHandle {
            path: constructed_path.into(),
            flags: deduced_flags.into(),
        };

        // We can get attributes directly from the directory item we just made
        let attributes: FileAttr = resulting_item.try_into()?;

        // Allocate the handle for it
        let handle_num: u64 = new_handle.allocate();

        // Assemble it, and we're done!
        debug!("Done creating file.");
        Ok(CreatedEntry {
            ttl: Duration::from_secs(60*60*24*365), // A year sounds good.
            attr: attributes,
            fh: handle_num,
            flags,
        })
    }
}
