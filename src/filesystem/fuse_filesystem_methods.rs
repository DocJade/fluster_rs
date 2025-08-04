// The actual FUSE filesystem layer.

//
//
// ======
// Imports
// ======
//
//

use fuse_mt::FilesystemMT;
use libc::c_int;
use bitflags::bitflags;
use log::info;

use crate::{filesystem::filesystem_struct::FlusterFS, pool::disk::generic::io::cache::cache_io::CachedBlockIO};

//
//
// ======
// Error types
// ======
//
//

const UNIMPLEMENTED: c_int = libc::ENOSYS;

//
//
// ======
// Handle type
// ======
//
//


// We are in charge of our own file handle management. Fun! (lie)
// So we need a way to hand out and retrieve them.


/// Handle for any type of item (file or directory).
struct ItemHandle {
    /// The path of this file/folder.
    path: Box<std::path::Path>, // Non-static size, thus boxed.
    /// Is this a file, or a directory?
    is_file: bool,
    // todo
}

impl ItemHandle {
    /// The name of the file/folder.
    fn name(&self) -> Option<&str> {
        // Get the name, if it exists.
        todo!()
    }

    /// Make a brand new file handle.
    fn make_handle(self) -> u64 {
        todo!()
    }

    /// Make a brand new file handle.
    fn read_handle(handle: u64) -> Self {
        todo!()
    }

    /// Release a handle.
    fn drop_handle(handle: u64) {
        todo!()
    }
}

//
//
// ======
// Flag type
// ======
//
//

// Flags are handled with bare u32 integers,
// hence we have a bitflag type to make dealing with them easier.

bitflags! {
    /// Flags that items have.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ItemFlag: u32 {
        // todo
    }
}

/// Convert a flag to a u32 for use in returning.
impl From<ItemFlag> for u32 {
    fn from(value: ItemFlag) -> Self {
        value.bits()
    }
}

/// Convert a u32 into a flag.
impl From<u32> for ItemFlag {
    fn from(value: u32) -> Self {
        // All bits must be used. We need to know what they all are.
        ItemFlag::from_bits(value).expect("All bits should be documented.")
    }
}

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
        info!("Done.");
        info!("Goodbye! .o/");
    }

    // Get file attributes.
    fn getattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
    ) -> fuse_mt::ResultEntry {
        Err(UNIMPLEMENTED)
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
    fn truncate(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _size: u64,
    ) -> fuse_mt::ResultEmpty {
        Err(UNIMPLEMENTED)
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

    // Create a new directory.
    fn mkdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _mode: u32,
    ) -> fuse_mt::ResultEntry {
        Err(UNIMPLEMENTED)
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
    fn open(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _flags: u32,
    ) -> fuse_mt::ResultOpen {
        Err(UNIMPLEMENTED)
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
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _flags: u32,
    ) -> fuse_mt::ResultCreate {
        Err(UNIMPLEMENTED)
    }
}
