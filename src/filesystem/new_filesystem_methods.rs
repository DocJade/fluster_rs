// The actual FUSE filesystem layer.

//
//
// ======
// Imports
// ======
//
//

use fuse_mt::FilesystemMT;

use crate::filesystem::filesystem_struct::FlusterFS;

//
//
// ======
// Error types
// ======
//
//

//
//
// ======
// Implementations
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
    fn init(&self, _req: fuse_mt::RequestInfo) -> fuse_mt::ResultEmpty {
        Ok(())
    }

    fn destroy(&self) {
        // Nothing.
    }

    fn getattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
    ) -> fuse_mt::ResultEntry {
        Err(libc::ENOSYS)
    }

    fn chmod(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _mode: u32,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn chown(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _uid: Option<u32>,
        _gid: Option<u32>,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn truncate(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _size: u64,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn utimens(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _atime: Option<std::time::SystemTime>,
        _mtime: Option<std::time::SystemTime>,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn utimens_macos(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn readlink(&self, _req: fuse_mt::RequestInfo, _path: &std::path::Path) -> fuse_mt::ResultData {
        Err(libc::ENOSYS)
    }

    fn mknod(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _rdev: u32,
    ) -> fuse_mt::ResultEntry {
        Err(libc::ENOSYS)
    }

    fn mkdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _mode: u32,
    ) -> fuse_mt::ResultEntry {
        Err(libc::ENOSYS)
    }

    fn unlink(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn rmdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn symlink(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _target: &std::path::Path,
    ) -> fuse_mt::ResultEntry {
        Err(libc::ENOSYS)
    }

    fn rename(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _newparent: &std::path::Path,
        _newname: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn link(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _newparent: &std::path::Path,
        _newname: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEntry {
        Err(libc::ENOSYS)
    }

    fn open(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _flags: u32,
    ) -> fuse_mt::ResultOpen {
        Err(libc::ENOSYS)
    }

    fn read(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _offset: u64,
        _size: u32,
        callback: impl FnOnce(fuse_mt::ResultSlice<'_>) -> fuse_mt::CallbackResult,
    ) -> fuse_mt::CallbackResult {
        callback(Err(libc::ENOSYS))
    }

    fn write(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _offset: u64,
        _data: Vec<u8>,
        _flags: u32,
    ) -> fuse_mt::ResultWrite {
        Err(libc::ENOSYS)
    }

    fn flush(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _lock_owner: u64,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn release(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn fsync(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _datasync: bool,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn opendir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _flags: u32,
    ) -> fuse_mt::ResultOpen {
        Err(libc::ENOSYS)
    }

    fn readdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
    ) -> fuse_mt::ResultReaddir {
        Err(libc::ENOSYS)
    }

    fn releasedir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _flags: u32,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn fsyncdir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _fh: u64,
        _datasync: bool,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn statfs(&self, _req: fuse_mt::RequestInfo, _path: &std::path::Path) -> fuse_mt::ResultStatfs {
        Err(libc::ENOSYS)
    }

    fn setxattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _name: &std::ffi::OsStr,
        _value: &[u8],
        _flags: u32,
        _position: u32,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn getxattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _name: &std::ffi::OsStr,
        _size: u32,
    ) -> fuse_mt::ResultXattr {
        Err(libc::ENOSYS)
    }

    fn listxattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _size: u32,
    ) -> fuse_mt::ResultXattr {
        Err(libc::ENOSYS)
    }

    fn removexattr(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn access(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        _mask: u32,
    ) -> fuse_mt::ResultEmpty {
        Err(libc::ENOSYS)
    }

    fn create(
        &self,
        _req: fuse_mt::RequestInfo,
        _parent: &std::path::Path,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _flags: u32,
    ) -> fuse_mt::ResultCreate {
        Err(libc::ENOSYS)
    }
}
