// I might blow a fuse.

// At this level of abstraction, we make calls to the Pool type. Nothing lower.

// Imports

use super::filesystem_struct::FLOPPY_PATH;
use super::filesystem_struct::FilesystemOptions;
use super::filesystem_struct::FlusterFS;
use super::filesystem_struct::USE_VIRTUAL_DISKS;
use crate::pool::pool_actions::pool_struct::Pool;
use easy_fuser::{FuseHandler, templates::DefaultFuseHandler};
use log::debug;
use std::path::PathBuf;
use std::process::exit;

// Implementations

impl FlusterFS {
    /// Create new filesystem handle, this will kick off the whole process of loading in information about the pool.
    /// Takes in options to configure the new pool.
    pub fn start(options: &FilesystemOptions) -> Self {
        debug!("Starting file system...");
        // Right now we dont use the options for anything, but they do initialize the globals we need, so we still need to pass it in.
        #[allow(dead_code)]
        #[allow(unused_variables)]
        let unused = options;
        let fs = FlusterFS {
            inner: Box::new(DefaultFuseHandler::new()),
            pool: Pool::load(),
        };
        debug!("Done starting filesystem.");
        fs
    }
}

// Now for the actual FUSE layer
//
// There's a lot of stuff in here we technically dont need. And I'm going to assume the information on this page is correct
// https://www.cs.hmc.edu/~geoff/classes/hmc.cs135.201001/homework/fuse/fuse_doc.html
// I have archived this page on internet archive.
// Thanks Geoff! I hope your life is going well, 16 years later.

impl FilesystemOptions {
    /// Initializes options for the filesystem, also configures the virtual disks if needed.
    pub fn new(use_virtual_disks: Option<PathBuf>, floppy_drive: PathBuf) -> Self {
        debug!("Configuring file system options...");
        // Set the globals
        // set the floppy disk path
        debug!("Setting the floppy path...");
        debug!("Locking FLOPPY_PATH...");
        *FLOPPY_PATH
            .try_lock()
            .expect("Fluster! Is single threaded.") = floppy_drive.clone();
        debug!("Done.");

        // Set the virtual disk flag if needed
        if let Some(path) = use_virtual_disks.clone() {
            debug!("Setting up virtual disks...");
            // Sanity checks
            // Make sure this is a directory, and that the directory already exists
            if !path.is_dir() || !path.exists() {
                // Why must you do this
                println!("Virtual disk argument must be a valid path to a pre-existing directory.");
                exit(-1);
            }

            debug!("Locking USE_VIRTUAL_DISKS...");
            *USE_VIRTUAL_DISKS
                .try_lock()
                .expect("Fluster! Is single threaded.") = Some(path.to_path_buf());
            debug!("Done.");
        };

        debug!("Done configuring.");
        Self {
            use_virtual_disks,
            floppy_drive,
        }
    }
}

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
        std::time::Duration::from_secs(10) // we're slow okay
    }

    // There's a few things we need to tweak in the KernelConfig.
    // This should be automatically called right when the FS starts if I'm reading the docs correctly.
    fn init(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        config: &mut easy_fuser::prelude::KernelConfig,
    ) -> easy_fuser::prelude::FuseResult<()> {

        // We don't want to stall for too long while doing writes, so we will set a max
        // write size of 1MB.
        // 
        // This value is in bytes.
        let _ = config.set_max_write(1024 * 1024).expect("Max write size of 1MB is invalid?");

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
        let _ = config.set_max_readahead(1).expect("Checked the implementation, this requires at least 1.");

        Ok(())
    }

    // fn destroy(&self) {
    //     self.get_inner().destroy();
    // }

    // "This call is not required but is highly recommended." Okay then we wont do it muhahaha
    // fn access(
    //     &self,
    //     req: &easy_fuser::prelude::RequestInfo,
    //     file_id: PathBuf,
    //     mask: easy_fuser::prelude::AccessMask,
    // ) -> easy_fuser::prelude::FuseResult<()> {
    //     self.get_inner().access(req, file_id, mask)
    // }

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
    // Exercise for the reader, dir_fs where all files are just directories in disguise
    fn create(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        flags: easy_fuser::prelude::OpenFlags,
    ) -> easy_fuser::prelude::FuseResult<(
        easy_fuser::prelude::OwnedFileHandle,
        <PathBuf as easy_fuser::prelude::FileIdType>::Metadata,
        easy_fuser::prelude::FUSEOpenResponseFlags,
    )> {
        todo!();
        // self.get_inner().create(
        //     req,
        //     parent_id,
        //     name,
        //     mode,
        //     umask,
        //     easy_fuser::prelude::flags,
        // )
    }

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

    /// Flushing information about open files / the file system to disk.
    /// We dont hold information about the filesystem in memory. If you wrote something, it already hit disk.
    fn flush(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        _file_id: PathBuf,
        _file_handle: easy_fuser::prelude::BorrowedFileHandle,
        _lock_owner: u64,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // bro idgaf
        Ok(())
    }

    // fn forget(&self, req: &easy_fuser::prelude::RequestInfo, file_id: PathBuf, nlookup: u64) {
    //     self.get_inner().forget(req, file_id, nlookup);
    // }

    // Sync a file to disk
    fn fsync(
        &self,
        _req: &easy_fuser::prelude::RequestInfo,
        _file_id: PathBuf,
        _file_handle: easy_fuser::prelude::BorrowedFileHandle,
        _datasync: bool,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // See flush()
        Ok(())
    }

    // Sync a whole directory
    fn fsyncdir(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::BorrowedFileHandle,
        datasync: bool,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // See flush()
        Ok(())
    }

    // "This call is pretty much required for a usable filesystem." okay fine.
    fn getattr(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: Option<easy_fuser::prelude::BorrowedFileHandle>,
    ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::FileAttribute> {
        todo!();
        // self.get_inner()
        //     .getattr(req, file_id, easy_fuser::prelude::file_handle)
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
    // I think this is where we can cram some caching in by checking if the full path is already referenced
    // in the cache section of the pool disk... We'll see.
    fn lookup(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
    ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    {
        todo!();
        // self.get_inner().lookup(req, parent_id, name)
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
        mode: u32,
        umask: u32,
    ) -> easy_fuser::prelude::FuseResult<<PathBuf as easy_fuser::prelude::FileIdType>::Metadata>
    {
        todo!();
        // self.get_inner().mkdir(req, parent_id, name, mode, umask)
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
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        flags: easy_fuser::prelude::OpenFlags,
    ) -> easy_fuser::prelude::FuseResult<(
        easy_fuser::prelude::OwnedFileHandle,
        easy_fuser::prelude::FUSEOpenResponseFlags,
    )> {

        // To disable any caching or reading ahead by the linux kernel (see init()) we need to
        // tell fuse that every file it opens is direct io with with the `FOPEN_DIRECT_IO` flag!
        // We also must support seeking, so the file handle that we give out needs to have a seek index.
        // TODO: see above!

        todo!();
        // self.get_inner()
        //     .open(req, file_id, easy_fuser::prelude::flags)
    }

    // "Open a directory for reading." Thanks Geoff, I think I got that.
    // I mean, feels pretty fundamental... We'll do it I guess...
    fn opendir(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        flags: easy_fuser::prelude::OpenFlags,
    ) -> easy_fuser::prelude::FuseResult<(
        easy_fuser::prelude::OwnedFileHandle,
        easy_fuser::prelude::FUSEOpenResponseFlags,
    )> {
        todo!();
        // self.get_inner()
        //     .opendir(req, file_id, easy_fuser::prelude::flags)
    }

    // "Read `size` bytes from the given file into the buffer `buf`, beginning offset bytes into the file."
    // "Required for any sensible filesystem." Yeah I bet.
    // It looks like we don't have some async buffer to write into, so we'll have to write the whole thing.
    // Although I did see whispers of a max read size setting... if we set that small-ish, then there shouldn't
    // be a massive latency problem from trying to load 30MB files or something.
    // What's a reasonable max though? 1MB? We'll see.
    fn read(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::BorrowedFileHandle,
        seek: std::io::SeekFrom,
        size: u32,
        flags: easy_fuser::prelude::FUSEOpenFlags,
        lock_owner: Option<u64>,
    ) -> easy_fuser::prelude::FuseResult<Vec<u8>> {
        todo!();
        // self.get_inner().read(
        //     req,
        //     file_id,
        //     easy_fuser::prelude::file_handle,
        //     seek,
        //     size,
        //     easy_fuser::prelude::flags,
        //     lock_owner,
        // )
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
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::BorrowedFileHandle,
    ) -> easy_fuser::prelude::FuseResult<
        Vec<(
            std::ffi::OsString,
            <PathBuf as easy_fuser::prelude::FileIdType>::MinimalMetadata,
        )>,
    > {
        todo!();
        // self.get_inner()
        //     .readdir(req, file_id, easy_fuser::prelude::file_handle)
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
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::OwnedFileHandle,
        flags: easy_fuser::prelude::OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
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
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::OwnedFileHandle,
        flags: easy_fuser::prelude::OpenFlags,
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
        req: &easy_fuser::prelude::RequestInfo,
        parent_id: PathBuf,
        name: &std::ffi::OsStr,
    ) -> easy_fuser::prelude::FuseResult<()> {
        // self.get_inner().rmdir(req, parent_id, name)
        todo!();
    }

    // This HAS to be mandatory right?
    // Linux > Yo what kinda file is this
    // Fluster! > Boi if you don't shut yo flightless bird ass up ima whoop yo ass
    // *Linux has SIGKILL'ed Fluster!*
    //
    // Yes we 100% need this, since we need to track file size changes and stuff.
    fn setattr(
        &self,
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        attrs: easy_fuser::prelude::SetAttrRequest,
    ) -> easy_fuser::prelude::FuseResult<easy_fuser::prelude::FileAttribute> {
        todo!();
        // self.get_inner().setattr(req, file_id, attrs)
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
        req: &easy_fuser::prelude::RequestInfo,
        file_id: PathBuf,
        file_handle: easy_fuser::prelude::BorrowedFileHandle,
        seek: std::io::SeekFrom,
        data: Vec<u8>,
        write_flags: easy_fuser::prelude::FUSEWriteFlags,
        flags: easy_fuser::prelude::OpenFlags,
        lock_owner: Option<u64>,
    ) -> easy_fuser::prelude::FuseResult<u32> {
        todo!();
        // self.get_inner().write(
        //     req,
        //     file_id,
        //     easy_fuser::prelude::file_handle,
        //     seek,
        //     data,
        //     write_flags,
        //     easy_fuser::prelude::flags,
        //     lock_owner,
        // )
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
