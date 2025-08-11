// The actual FUSE filesystem layer.

//
//
// ======
// Imports
// ======
//
//

use std::{path::Path, time::Duration};

use fuse_mt::{DirectoryEntry, FileAttr, FileType, FilesystemMT};
use log::{debug, error, info, warn};

use crate::{
    filesystem::{
        filesystem_struct::FlusterFS,
        item_flag::flag_struct::ItemFlag
    },
    pool::{disk::{
        generic::io::cache::cache_io::CachedBlockIO,
        standard_disk::block::{
            directory::directory_struct::{
                DirectoryBlock,
                DirectoryFlags,
                DirectoryItem
            },
            io::directory::types::NamedItem
        }
    }, pool_actions::pool_struct::Pool}
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
        CachedBlockIO::flush().expect("I sure hope cache flushing works!");
        // Now flush pool information
        info!("Flushing pool info...");
        Pool::flush().expect("I sure hope pool flushing works!");
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

        Ok(
            (
                a_year,
                found_attributes
            )
        )
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
        let the_name: String = name.to_str().expect("Should be valid utf8").to_string();

        // Open parent
        if let Some(mut parent) = DirectoryBlock::try_find_directory(Some(parent))? {
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
        parent: &std::path::Path,
        name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        debug!("Deleting file `{}` from directory `{}`...", name.display(), parent.display());


        let the_name: String = name.to_str().expect("Should be valid utf8").to_string();

        // Ensure this is not a directory
        let temp_handle: FileHandle = FileHandle {
            path: parent.join(name).into(),
            flags: ItemFlag::empty(),
        };

        if !temp_handle.is_file() {
            // Cannot unlink directories.
            debug!("A directory was provided, not a file.");
            return Err(NOT_A_DIRECTORY);
        }


        // Open directory
        debug!("Looking for file...");
        if let Some(mut parent_dir) = DirectoryBlock::try_find_directory(Some(parent))? {
            // dir exists, does the file?
            if let Some(the_file) = parent_dir.find_item(&NamedItem::File(the_name))? {
                // File exists, delete it.
                if parent_dir.delete_file(the_file.into())?.is_some() {
                    // All done.
                    Ok(())
                } else {
                    // Weird, we checked that the directory was there, but when we went to delete it, it wasnt???
                    warn!("We found the directory to delete, but when we tried to delete it, it was missing.");
                    // this should not happen lmao, but whatever.
                    Err(NO_SUCH_ITEM)
                }
            } else {
                // No such file.
                debug!("File does not exist.");
                Err(NO_SUCH_ITEM)
            }
        } else {
            // bad folder
            debug!("Parent folder does not exist.");
            Err(NO_SUCH_ITEM)
        }
    }

    // Deletes a directory.
    // Should fail if the directory is not empty.
    fn rmdir(
        &self,
        _req: fuse_mt::RequestInfo,
        parent: &std::path::Path,
        name: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        debug!("Attempting to remove directory `{}` from `{}`...", name.display(), parent.display());

        let string_name: String = name.to_str().expect("Should be valid utf8").to_string();

        // Open the parent directory
        if let Some(parent_dir) = DirectoryBlock::try_find_directory(Some(parent))? {
            // Parent exists, get the child
            if let Some(child_dir) = parent_dir.find_item(&NamedItem::Directory(string_name))? {
                // Directory exists.

                // Make sure this is actually a directory
                if !child_dir.flags.contains(DirectoryFlags::IsDirectory) {
                    // Not a dir
                    debug!("Provided item is not a directory.");
                    return Err(NOT_A_DIRECTORY);
                }

                // Get the block
                let block_to_delete = child_dir.get_directory_block()?;

                // Make sure it's empty
                if !block_to_delete.is_empty()? {
                    // Nope.
                    debug!("Directory is not empty, cannot delete.");
                    return Err(DIRECTORY_NOT_EMPTY);
                }

                // Run the deletion.
                debug!("Deleting directory...");
                block_to_delete.delete_self(child_dir)?;
                debug!("Done.");
                Ok(())
                
            } else {
                // child directory did not exist.
                debug!("The directory we wanted to delete does not exist.");
                Err(NO_SUCH_ITEM)
            }
        } else {
            // parent dir went to get milk
            debug!("Parent directory does not exist.");
            Err(NO_SUCH_ITEM)
        }
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

    // Renames / moves item.
    // Complicated error logic due to https://man7.org/linux/man-pages/man2/rename.2.html
    fn rename(
        &self,
        _req: fuse_mt::RequestInfo,
        parent: &std::path::Path,
        name: &std::ffi::OsStr,
        newparent: &std::path::Path,
        newname: &std::ffi::OsStr,
    ) -> fuse_mt::ResultEmpty {
        debug!("Renaming a item from `{}` to `{}`,", name.display(), newname.display());
        debug!("and moving from `{}` to `{}`.", parent.display(), newparent.display());

        // According to the man pages, we should get some flags here. but we dont.
        // I assume things like RENAME_NOREPLACE are being handled for us then.

        // Any case:
        // EISDIR newpath is an existing directory, but oldpath is not a directory.
        // EINVAL The new pathname contained a path prefix of the old, or, more generally, an attempt was made to make a directory a
        //  subdirectory of itself. (recursion moment)
        // ENOENT The link named by oldpath does not exist; or, a directory component in newpath does not exist; or, oldpath or newpath
        //  is an empty string. (TLDR this is a catch all)

        // If we are moving a directory:
        // The new path must not exist, or be empty.
        // ENOTEMPTY or EEXIST newpath is a nonempty directory
        // ENOTDIR A component used as a directory in oldpath or newpath is not, in fact, a directory.  Or, oldpath is a directory, and
        //  newpath exists but is not a directory. (Cant move non directories into directories, cant move directory into non directory.)

        // "If newpath exists but the operation fails for some reason,
        //  rename() guarantees to leave an instance of newpath in place."
        // why word it like this lmao
        // if the destination already exists, but the move fails, keep what was already at the destination.

        // Also in theory, we should be checking if anyone is reading this item and if they are, return busy.
        // but there isnt any infra for that yet, and with the one year timeouts, you would have to wait a while.
        // fun. we will ignore it until something explodes.



        // This is gonna be REALLY complicated. lol.
        
        

        // Make sure the new name isn't too long
        if newname.len() > 255 {
            // too long
            warn!("New item name was too long");
            return Err(FILE_NAME_TOO_LONG);
        }


        // Now, the "easy" cases.
        // Where we're coming from (including name of file/folder)
        let source_full_temp_handle: FileHandle = FileHandle {
            path: parent.join(name).into(),
            flags: ItemFlag::empty(),
        };

        // Where we want to go
        // Where we're coming from (including name of file/folder)
        let destination_full_temp_handle: FileHandle = FileHandle {
            path: newparent.join(newname).into(),
            flags: ItemFlag::empty(),
        };

        // If they are the same, we dont need to do anything at all.
        if source_full_temp_handle.path == destination_full_temp_handle.path {
            // why...
            debug!("Source and destination are the same, skipping.");
            return Ok(());
        }

        // Make sure the two are the same underlying type
        debug!("Making sure the two are of the same type...");
        if source_full_temp_handle.is_file() == destination_full_temp_handle.is_file(){
            debug!(
                "Types are the same, both are {}.",
                if source_full_temp_handle.is_file() {
                    "files"
                } else {
                    "directories"
                }
            )
            // They are both the same.
        } else {
            // Types are different, we cannot do that
            warn!("Types are different, we cannot perform this rename/move.");
            return Err(NOT_A_DIRECTORY);
        }

        // Now that we know the two types are the same,
        // Grab the parents and the item we are attempting to move depending on type.

        // For both types of rename/move operations, we must have:
        // - The parent folder of the source, and destination
        // - The item we are renaming
        // But we do not need the destination item to exist.
        // Any logic around that is handled differently depending on if this was a file or not.

        // I wrote directory handling first, then came back for file movement. Standing on the shoulders of myself, i know that
        // directories always return a Err(NO_SUCH_ITEM) if either of the parents do not exist. This is also true of files, so we
        // can perform that check out here.
        
        debug!("Trying to obtain the parents of the source and destination, and the directory item for the item we are trying to move.");

        let source_item_name: String = name.to_str().expect("Should be valid utf8").to_string();
        let destination_item_name: String = newname.to_str().expect("Should be valid utf8").to_string();

        debug!("Checking if parents existed...");
        // Try to get the source parent directory, then try to get the item refering to the dir we are moving.
        let mut source_parent_dir: DirectoryBlock = if let Some(exist) = DirectoryBlock::try_find_directory(Some(parent))? {
            // Good.
            debug!("Source parent exists.");
            exist
        } else {
            // missing
            warn!("Source parent did not exist. Cannot continue.");
            return Err(NO_SUCH_ITEM);
        };

        let mut destination_parent_dir: DirectoryBlock = if let Some(exist) = DirectoryBlock::try_find_directory(Some(newparent))? {
            // Good.
            debug!("Destination parent exists.");
            exist
        } else {
            // missing
            warn!("Destination parent did not exist. Cannot continue.");
            return Err(NO_SUCH_ITEM);
        };

        // Item logic must be handled lower down, but we can at least abstract the calls out at this point to work with options later.

        // We know the kind here so we can abstract this away as well.
        let maybe_source_directory_item: Option<DirectoryItem>;
        let maybe_destination_directory_item: Option<DirectoryItem>;
        if source_full_temp_handle.is_file() {
            // both files.
            maybe_source_directory_item = source_parent_dir.find_item(&NamedItem::File(source_item_name.clone()))?;
            maybe_destination_directory_item = destination_parent_dir.find_item(&NamedItem::File(destination_item_name.clone()))?;
        } else {
            // both directories.
            maybe_source_directory_item = source_parent_dir.find_item(&NamedItem::Directory(source_item_name.clone()))?;
            maybe_destination_directory_item = destination_parent_dir.find_item(&NamedItem::Directory(destination_item_name.clone()))?;
        };

        // The following complicated move logic requires that the two parent directories be different. If the source and 
        // destination directories are the same, we can just rename the inode, skipping all of the fancier operations.

        if parent == newparent {
            // Sweet!
            // We dont even need the destination info
            drop(maybe_destination_directory_item);
            drop(destination_full_temp_handle);
            drop(destination_parent_dir);

            // Source must exist.
            let source = match maybe_source_directory_item {
                Some(ok) => ok,
                None => {
                    // Can't rename nothing.
                    return Err(NO_SUCH_ITEM);
                },
            };

            // Type does not matter, we can just update the name in the directory, since inodes do not hold that info.
            // Explicitly check the error, silently returning when this fails is bad.
            let rename_result = match source_parent_dir.try_rename_item(&source.into(), destination_item_name) {
                Ok(ok) => ok,
                Err(err) => {
                    // Renaming failed for a lower level issue!
                    warn!("Item rename failed! Why?");
                    warn!("`{err:#?}`");
                    // Bail out
                    return Err(err.into());
                },
            };


            if rename_result {
                // rename worked.
                debug!("Item renamed successfully.");
                return Ok(())
            } else {
                // Somehow the item was not there anymore? This should never happen.
                unreachable!("Item to rename disappeared!")
            }
        }

        // This rename moves the item between directories.


        // we branch depending on if it was a file or directory, handling is slightly different
        if source_full_temp_handle.is_file() {
            //
            // File movement.
            //
            debug!("Starting file movement...");
            // If the new item exists, it will be replaced. (see rename(2) manpage)

            // The source item must exist.
            debug!("Checking that source item exists...");
            let source_item = if let Some(existed) = maybe_source_directory_item {
                // good
                debug!("Yes it does.");
                existed
            } else {
                // cant move nothing.
                warn!("Source item did not exist. Cannot perform rename/move.");
                return Err(NO_SUCH_ITEM);
            };

            debug!("Checking if destination file already existed...");
            // Check if the destination file already exists, that will change our behavior on failure.
            if maybe_destination_directory_item.is_some() {
                // Destination item exists, we will be overwriting this, but we will hold onto it just in case.
                // In theory we should try to put this back if the rename fails.

                // Since the item exists already, we will extract it. To delete the old file we need to have the file in the block. Which
                // complicates things a bit. So we pull it out, and when we are ready to delete it, we rename it, put it back in, and delete it.
                let extracted_old = match destination_parent_dir.find_and_extract_item(&NamedItem::Directory(destination_item_name.clone())) {
                    Ok(ok) => match ok {
                        Some(ok) => ok,
                        None => {
                            // We tried to extract it, but it was no longer there?
                            // No data has been modified (by us here at least), entire operation can be retried.
                            warn!("Destination item was no longer there when extraction was attempted. Non-fatal, but weird.");
                            return Err(NO_SUCH_ITEM)
                        },
                    },
                    Err(error) => {
                        // The extraction failed at _some_ point, unknown where. Chances are the destination item is now gone. But we have failed.
                        warn!("Extracting the old item failed for a low level reason, we have to bail.");
                        return Err(error.into())
                    },
                };

                // Now we have that copy for later, we can stick a copy of the source item into the destination
                debug!("Inserting copy of the source file into the destination directory...");
                // We must give it the new name first:
                let mut renamed_source_item = source_item;
                renamed_source_item.name = destination_item_name.clone();
                renamed_source_item.name_length = destination_item_name.len() as u8; // Already checked name length.
                match destination_parent_dir.add_item(&renamed_source_item) {
                    Ok(_) => (),
                    Err(error) => {
                        // Failed to add item to destination
                        warn!("Failed to put copy into the destination for low level reason.");
                        // Try to put back the file that was here before.
                        warn!("Attempting to restore previous item if possible...");
                        // Chances are if the previous one failed, this will to, but whatever.
                        if destination_parent_dir.add_item(&extracted_old).is_ok() {
                            warn!("Previous file restored.");
                        } else {
                            // oof
                            warn!("Failed to restore previous file. File has been permanently lost.");
                        }
                        // As good as it'll get. Return the error that caused us to fail.
                        return Err(error.into())
                    },
                }
                debug!("Done.");
                debug!("Removing old item from origin directory...");

                // Now go to the parent of the source and tell em to slime tf outta the old item

                match source_parent_dir.find_and_extract_item(&NamedItem::File(source_item_name.clone())) {
                    Ok(ok) => {
                        match ok {
                            Some(_found) => {
                                // Removal worked
                                debug!("Source item removed.")
                            },
                            None => {
                                // Item we tried to remove was not there.
                                // Weird, but if we got this far, it must have at least made it into the destination.
                                warn!("The source item we tried to remove was not there. Which is fine, since that was the goal anyways.");
                            },
                        }
                    },
                    Err(err) => {
                        // We have now failed to remove the previous item, but the new one is in place. This is good enough.
                        // If the item is still in that other block, it is now a duplicate reference of the underlying file.
                        // Not great... but not much I can do here. No transactions means no safe actions!
                        warn!("Failed to remove previous item, it may still be there. Good enough.");
                        // The move has now technically finished, even though we have errored at this point, so guess what?
                        debug!("Removal failed due to: {err:#?}");
                        return Ok(());
                    },
                };

                // Now that the file is no longer where it used to live, we can delete the item that used to live in the destination that
                // we overwrote.

                // If this fails, the item is no longer in the file, but will still occupy blocks on disk, which isn't great, but at least
                // you cant get to them.

                // Rename the item so we dont collide with the newly moved in file
                let mut renamed: DirectoryItem = extracted_old;
                renamed.name.push_str(".fluster_old");
                // Is this too long now?
                if renamed.name.len() > 255 {
                    // Shoot, go with a stupid name instead.
                    // Yes this could collide. If it does, we are cooked. Good luck!
                    // TODO: Consider using a hash of the name, or just a UUID?
                    renamed.name = "DocJadeWasHereAndNeededToDeleteThis.delete_me".to_string();
                    renamed.name_length = renamed.name.len() as u8; // will fit
                }

                // Hold onto the new name for later
                let deletion_name: String = renamed.name.clone();

                // Put the renamed item in there.
                debug!("Re-inserting old file with a new name to delete it...");
                match destination_parent_dir.add_item(&renamed) {
                    Ok(_) => {
                        // Adding worked.
                        debug!("Inserted.")
                    },
                    Err(err) => {
                        // Damn. We will just leak the blocks this took up.
                        warn!("Insertion failed.");
                        warn!("Just to keep going, we will leak the blocks that the old file references.");
                        warn!("Rename \"finished\".");
                        debug!("Failed due to: {err:#?}");
                        // Rename still worked overall.
                        return Ok(());
                    },
                }
                // Now kill it
                debug!("Deleting the old file...");
                match destination_parent_dir.delete_file(NamedItem::File(deletion_name)) {
                    Ok(ok) => match ok {
                        Some(_) => {
                            // File was deleted.
                            debug!("Old file deleted.")
                        },
                        None => {
                            // The file did not exist?!?
                            warn!("Somehow, we added the file, and when we went to delete it, it no longer existed.");
                            warn!("It probably leaked, but there is nothing we can do.");
                            // Good enough!
                            warn!("Good enough. Rename finished.");
                            return Ok(());
                        },
                    },
                    Err(err) => {
                        // Yet another leak scenario.
                        warn!("Deletion failed.");
                        warn!("Just to keep going, we will leak the blocks that the old file references.");
                        warn!("Rename \"finished\".");
                        debug!("Failed due to: {err:#?}");
                        // Rename still worked overall.
                        return Ok(());
                    },
                }

                // File has been deleted. Cleanup is now finished.
                // Done moving file, which replaced a pre-existing file.
            } else {
                // We are not trying to overwrite a pre-existing file, this makes our lives easier.
                debug!("Destination item does not exist yet. We will create it.");

                // We must give it the new name first:
                let mut renamed_source_item = source_item;
                renamed_source_item.name = destination_item_name.clone();
                renamed_source_item.name_length = destination_item_name.len() as u8; // Already checked name length.

                // Put the file into the destination
                debug!("Adding source file to destination directory...");
                match destination_parent_dir.add_item(&renamed_source_item) {
                    Ok(_) => {
                        // That worked
                    },
                    Err(err) => {
                        // Failed to add to destination
                        // Drive level issue.
                        warn!("Failed at a level lower than us. Unknown state.");
                        debug!("Failed due to: {err:#?}");
                        return Err(err.into())
                    },
                }
                debug!("File added.");

                // Now we need to remove the old item.
                match source_parent_dir.delete_file(NamedItem::File(source_item_name)) {
                    Ok(ok) => {
                        // if ok is none, the item disappeared, which should not happen.
                        assert!(ok.is_some(), "File should not disappear.")
                    },
                    Err(err) => {
                        // The file made it to the destination, but removing the original failed.
                        // The old item may still be there, or it leaked blocks due to failed cleanup.
                        warn!("Failed to delete source item, it may still be there.");
                        warn!("Blocks were probably leaked.");
                        warn!("Non-critical failure, we will keep going.");
                        // Good enough.
                    },
                }
            }
            // All done.
            debug!("File moved successfully.");
            Ok(())
        } else {
            //
            // Directory movement.
            //
            debug!("Starting directory movement...");
            
            // Make sure we aren't trying to make a self referential folder
            debug!("Checking for recursion...");
            if destination_full_temp_handle.path.starts_with(&source_full_temp_handle.path) {
                // Destination contains the source, therefore this is self referential.
                warn!("Cannot move directory inside of itself.");
                return Err(INVALID_ARGUMENT);
            }
            debug!("No recursion.");

            
            // To fulfill the requirement of the destination directory being empty, we will check if the directory exists in the
            // parent. If it does, make sure its empty, if it is, then we will update the DirectoryItem to point at the block for
            // the start of the DirectoryBlocks of the source directory.
            // If the directory does not exist, we will create it and still do the same pointer swap.
            // if the directory is not empty, we have to cancel the move.

            // Do both directories exist?
            debug!("Checking if the parents contain directory item we want to move...");
            // source
            let source_directory_item: DirectoryItem = if let Some(item) = maybe_source_directory_item {
                // Source exists
                item
            } else {
                // Item was not there. Cant copy nothing.
                debug!("Source directory missing, cannot rename/move folder.");
                return Err(NO_SUCH_ITEM);
            };
            debug!("Source good.");

            // Check that the destination folder exists and is empty.
            if let Some(item) = maybe_destination_directory_item {
                // Destination has to be empty
                debug!("Destination already existed, making sure its empty...");
                if item.clone().get_directory_block()?.is_empty()? {
                    // All good, we will delete the directory since we are going to replace it.
                    debug!("It's empty, it will be deleted soon");
                } else {
                    // Directory was not empty, we cannot continue
                    warn!("Destination directory was not empty, cannot rename/move folder.");
                    return Err(DIRECTORY_NOT_EMPTY);
                }
            } else {
                // no destination, we will make it
                debug!("Destination did not exist, creating...");
                // Annoying clone.
                let _ = destination_parent_dir.make_directory(destination_item_name.clone())?;
                // yes we are creating it to just remove it again, but we are supposed to (in theory if i remember correctly idk its 1am) leave
                // a folder at the destination even if the move fails.
            };
            debug!("Destination good.");


            // Now for the fun part, we can extract the DirectoryItem from the first directory, and swap it into
            // the second one, thus the new folder points at the contents without moving the underlying files in the folder.

            debug!("Swapping DirectoryItems...");

            // Remove the destination folder
            // "Inshallah he will be grounded into a fine dust"

            // We have to tread lightly at this point. If the swap fails, we would lose data.
            // 🤓 erm actually the data would still be there, just not referenced- SHUT UP

            // Extract, and delete. Extraction cleans up anything this used to point to for us.
            debug!("Extracting destination...");
            let _extracted_dest = match destination_parent_dir.find_and_extract_item(&NamedItem::Directory(destination_item_name.clone())) {
                Ok(ok) => {
                    // Directory had to've been there, right?
                    if let Some(worked) = ok {
                        // Found and extracted the item, we will hold onto it incase we need to recreate it.
                        worked
                    } else {
                        // What
                        warn!("Tried to delete the destination directory to prepare for swap, but it was no longer there.");
                        // This should be impossible.
                        unreachable!();
                    }
                    // We will hold onto it just in case, even though it's empty.
                },
                Err(err) => {
                    // Drive level issue.
                    warn!("Failed at a level lower than us. Unknown state.");
                    return Err(err.into())
                },
            };
            debug!("Destination extracted");

            // Now get a COPY of the source, we wont remove the source until we know for sure we have properly moved the folder.
            // Wait, we already have it, in `source_directory_item`, duh
            
            // Attempt to add the directory to the new parent
            debug!("Inserting copy of source directory...");

            // Make sure we rename dat mf fr
            let mut renamed_source_dir_item = source_directory_item;
            renamed_source_dir_item.name = destination_item_name.clone();
            renamed_source_dir_item.name_length = destination_item_name.len() as u8; // Length checked above

            match destination_parent_dir.add_item(&renamed_source_dir_item) {
                Ok(_) => {
                    // that worked
                    // No need to do anything
                },
                Err(err) => {
                    // Drive level issue.
                    warn!("Failed at a level lower than us. Unknown state.");
                    // Attempt to uphold POSIX standard (like hell the rest of fluster is compliant) by
                    // at least attempting to put the original directory back again.
                    // We dont actually need to though, since it hasn't been extracted yet.
                    return Err(err.into())
                },
            }
            debug!("Insertion succeeded.");
            
            // Now that the data has been safely pointed at from the new location, we will remove the old reference to it.
            debug!("Removing old source...");
            let ashes = match source_parent_dir.find_and_extract_item(&NamedItem::Directory(source_item_name.clone())) {
                Ok(ash) => ash,
                Err(err) => {
                    // Drive level issue.
                    warn!("Failed at a level lower than us. Unknown state.");
                    // So now we have properly moved the source into the destination, but the source might still be there.
                    // This wouldn't be too bad, if not for the fact that now both DirectoryBlocks contain the same DirectoryItem which
                    // points to the same directory. Thus they have become hard-linked. Not great.
                    // Good luck lmao.
                    warn!("There isn't really anything we can do at this point, a hard link has been created due to this.");
                    warn!("We consider this good enough. Done moving directory.");
                    // shoulda thought ahead and made fluster more transactional, oh well.
                    // We have to ignore the error, but might as well print it.
                    debug!("{err:#?}");
                    return Ok(());
                },
            };


            if let Some(_to_ashes) = ashes {
                // It was there, and removed.
                // We're all done.
                debug!("Done.")
            } else {
                // ????? DIRECTORY IS NOT THERE (GONE) (STOLEM)
                warn!("Somehow, we copied the source directory across correctly, but now the source is missing so we cant remove it.");
                // I mean like, this still worked, no?
                // goals:
                // put source in destination: check
                // source is no longer there: check
                // sooooooo
                // GOOD ENOUGH!
                warn!("...but that's close enough.");
            };

            // All done!
            debug!("Rename finished, directories renamed/moved.");
            Ok(())
        }
        // this comment is unreachable, all cases are covered.
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
        let mut handle: FileHandle = FileHandle {
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

        // If this is the dot directory, we need to go up a level to read ourselves.
        if handle.name() == "." {
            // Go up a path.
            // If this returns none, all is well
            handle.path = handle.path.parent().unwrap_or(Path::new("")).into();
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
        debug!("Getting a handle on things...");
        let new_handle: u64 = handle.allocate();
        
        // Done!
        debug!("Opening finished.");
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
        path: &std::path::Path,
        fh: u64,
        offset: u64,
        size: u32,
        callback: impl FnOnce(fuse_mt::ResultSlice<'_>) -> fuse_mt::CallbackResult,
    ) -> fuse_mt::CallbackResult {
        debug!("Reading `{}` bytes from file `{}`", size, path.display());

        // Open the file handle
        let got_handle = FileHandle::read(fh);

        // Still not sure if we need to check this, but whatever.
        if got_handle.path != path.into() {
            // They aren't the same? not sure what to do with that
            error!("readdir() tried to read a path, but provided a handle to a different path.");
            error!("fh: `{}` | path: `{}`", got_handle.path.display(), path.display());
            error!("Not sure what to do here, giving up.");
            return callback(Err(GENERIC_FAILURE));
        }

        // Make sure this is a file
        if !got_handle.is_file() {
            // Can't read a directory!
            warn!("Tried to read a directory as a file. Ignoring...");
            return callback(Err(IS_A_DIRECTORY));
        }

        // Get the item
        let named = got_handle.get_named_item();

        // Try to find it.
        // Cant use the `?` operator in here due to the callback, annoying!
        let parent: DirectoryBlock = match DirectoryBlock::try_find_directory(got_handle.path.parent()) {
            Ok(ok) => match ok {
                Some(found) => found,
                None => {
                    // No such parent, therefore no such file.
                    debug!("No parent for file, returning...");
                    return callback(Err(NO_SUCH_ITEM))
                },
            },
            Err(error) => {
                // Lower level error
                return callback(Err(error.into()))
            },
        };
        
        // Is the file there?
        let file = match parent.find_item(&named) {
            Ok(ok) => match ok {
                Some(exists) => exists,
                None => {
                    // No such file.
                    debug!("No such file, returning...");
                    return callback(Err(NO_SUCH_ITEM))
                },
            },
            Err(error) => {
                // Lower level error
                warn!("Failed while finding item! Giving up...");
                return callback(Err(error.into()))
            },
        };

        // Found a file!
        // We need to bound our read by the size of the file, since the read() filesystem call can
        // try to read past the end.
        let file_size = match file.get_size() {
            Ok(ok) => ok,
            Err(error) => {
                // Lower level error
                warn!("Failed to get size of file! Giving up...");
                return callback(Err(error.into()))
            },
        };

        // Subtract the offset to idk man why am i explaining this im sure you understand.
        // Reads are limited to 4GB long, which should be way above our max read size anyways.
        let bounded_read_length:u32 = std::cmp::min(size as u64, file_size - offset).try_into().expect("Reads should not be >4GB.");
        if bounded_read_length != size {
            // size did change.
            debug!("Read was too large, truncated to `{bounded_read_length}` bytes.");
        }

        // Do the read.
        // This vec might be HUGE, this is why we need to limit the read size on the filesystem.
        debug!("Starting read...");
        let read_buffer: Vec<u8> = match file.read_file(offset, bounded_read_length) {
            Ok(ok) => ok,
            Err(error) => {
                // Lower level error
                warn!("Failed while reading the file! Giving up...");
                return callback(Err(error.into()))
            },
        };
        debug!("Read finished.");

        // All done!
        callback(Ok(&read_buffer))
    }

    // Write data to a file using a file handle.
    fn write(
        &self,
        _req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        fh: u64,
        offset: u64,
        data: Vec<u8>,
        _flags: u32, // hehe
    ) -> fuse_mt::ResultWrite {
        debug!("Writing `{}` bytes to file `{}`...", data.len(), path.display());

        // Open the file handle
        let got_handle = FileHandle::read(fh);

        // Still not sure if we need to check this, but whatever.
        if got_handle.path != path.into() {
            // They aren't the same? not sure what to do with that
            error!("readdir() tried to read a path, but provided a handle to a different path.");
            error!("fh: `{}` | path: `{}`", got_handle.path.display(), path.display());
            error!("Not sure what to do here, giving up.");
            return Err(GENERIC_FAILURE);
        }

        // Make sure this is a file
        if !got_handle.is_file() {
            // Can't read a directory!
            warn!("Tried to read a directory as a file. Ignoring...");
            return Err(INVALID_ARGUMENT); // write() man page
        }

        // Get the item
        let named = got_handle.get_named_item();

        // Try to find it.

        // Parent
        let parent = if let Some(found) = DirectoryBlock::try_find_directory(got_handle.path.parent())? {
            // Good
            found
        } else {
            // No such parent.
            debug!("No parent for file, returning...");
            return Err(NO_SUCH_ITEM)
        };

        // File
        let file = if let Some(found) = parent.find_item(&named)? {
            // Found it!
            found
        } else {
            // File is not there.
            debug!("No such file, returning...");
            return Err(NO_SUCH_ITEM)
        };

        
        // man page:
        // If count is zero and fd refers to a regular file, then write() may
        // return a failure status if one of the errors below is detected.
        // If no errors are detected, or error detection is not performed, 0
        // is returned without causing any other effect.  If count is zero
        // and fd refers to a file other than a regular file, the results are
        // not specified.
        //
        // So if we want to write zero bytes, do nothing
        if data.is_empty() {
            // uh ok then
            debug!("Caller wanted to write 0 bytes. Skipping write.");
            return Ok(0);
        }




        // Now write to the file!
        debug!("Starting write...");
        let bytes_written = file.write_file(&data, offset)?;
        debug!("Write completed.");

        // Make sure it all got written
        assert_eq!(bytes_written, data.len().try_into().expect("Should be less than a u32"));

        // Return the number of bytes written.
        Ok(bytes_written)
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

    // Releasing a file handle.
    fn release(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> fuse_mt::ResultEmpty {
        FileHandle::drop_handle(fh);
        Ok(())
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
        req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        flags: u32,
    ) -> fuse_mt::ResultOpen {

        // This just gets pushed over to open(), since
        // we already handle directories over there.
        //
        // Should we handle files and directories both in open? maybe not.
        self.open(req, path, flags)
    }

    // List the contents of a directory.
    // "Return one or more directory entries (struct dirent) to the caller."
    // "This is one of the most complex FUSE functions." Oof.
    // "The readdir function is somewhat like read, in that it starts at a
    //  given offset and returns results in a caller-supplied buffer."
    // "However, the offset not a byte offset" What the hell
    // "...and the results are a series of struct dirents rather than being uninterpreted bytes" those are just words Geoffery
    //
    // Luckily we are working at a level way above that!
    fn readdir(
        &self,
        _req: fuse_mt::RequestInfo,
        path: &std::path::Path,
        fh: u64,
    ) -> fuse_mt::ResultReaddir {
        debug!("Getting contents of directory `{}`...", path.display());

        // Make sure the file handle and the incoming path are the same. I assume they should be, but
        // cant hurt to check.
        let got_handle = FileHandle::read(fh);
        
        if got_handle.path != path.into() {
            // They aren't the same? not sure what to do with that
            error!("readdir() tried to read a path, but provided a handle to a different path.");
            error!("fh: `{}` | path: `{}`", got_handle.path.display(), path.display());
            error!("Not sure what to do here, giving up.");
            return Err(GENERIC_FAILURE);
        }

        // Since we have a handle, getting the directory is easy.
        debug!("Getting the directory item from handle...");
        let dir_item: DirectoryItem = if let Ok(exists) = got_handle.get_directory_item() {
            // good
            exists
        } else {
            // Tried to read in a directory item that did not exist, yet we have a handle to it?
            // Guess the handle must be stale?

            // Yes, get_directory_item() returns its own error, but we should get rid of the invalid handle.

            warn!("Tried to read in a directory item from a handle, but the item was not there. Returning stale.");
            return Err(STALE_HANDLE)
        };
        
        // Double check that this is a file.
        if !dir_item.flags.contains(DirectoryFlags::IsDirectory) {
            // No.
            warn!("Tried to call readdir on a file!");
            return Err(NOT_A_DIRECTORY);
        }
        
        debug!("Getting directory block...");
        let dir_block = dir_item.get_directory_block()?;

        // List the files off
        debug!("Listing items...");
        let items = dir_block.list()?;
        
        // Now pull out the names and types
        let mut listed_items: Vec<DirectoryEntry> = items.iter().map(|item| {
            let kind = if item.flags.contains(DirectoryFlags::IsDirectory) {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            
            DirectoryEntry {
                name: item.name.clone().into(),
                kind,
            }
        }).collect();

        // Now add the unix `.` item.
        listed_items.push(
            DirectoryEntry {
                name: std::ffi::OsStr::new(".").into(),
                kind: FileType::Directory,
            }
        );


        
        // All done!
        debug!("Done. Directory contained `{}` items.", listed_items.len());
        Ok(listed_items)
    }

    // See release()
    fn releasedir(
        &self,
        _req: fuse_mt::RequestInfo,
        _path: &std::path::Path,
        fh: u64,
        _flags: u32,
    ) -> fuse_mt::ResultEmpty {
        FileHandle::drop_handle(fh);
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
        let mut containing_dir_block: DirectoryBlock = match DirectoryBlock::try_find_directory(Some(parent))? {
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
        if let Some(exists) = containing_dir_block.find_item(&NamedItem::File(converted_name.clone()))? {
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

            // Truncate if needed (open(2) syscall)
            // Must be a file
            if deduced_flags.contains(ItemFlag::TRUNCATE) && !exists.flags.contains(DirectoryFlags::IsDirectory) {
                self.truncate(req, constructed_path, Some(file_handle), 0)?; // Truncate to 0
            }
            
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
        debug!("Created file.");

        // Full item path
        let constructed_path: &Path = &parent.join(name);

        // Construct and return the handle to the new file
        let new_handle: FileHandle = FileHandle {
            path: constructed_path.into(),
            flags: deduced_flags,
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
