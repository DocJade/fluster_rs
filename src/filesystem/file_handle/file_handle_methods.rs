// Make the handle do things.

use std::{collections::HashMap, sync::{Arc, Mutex}};

use lazy_static::lazy_static;
use libc::c_int;
use log::{debug, error};

//
// Global info about open files
//

struct LoveHandles {
    /// Hashmap of the currently allocated handles
    allocated: HashMap<u64, FileHandle>,
    /// Highest allocated number (is kept up to date internally)
    highest: u64,
    /// Recently freed handles (ie open space in the hashmap)
    free: Vec<u64>
}

impl LoveHandles {
    /// Make a new one, should only be called once.
    fn new() -> Self {
        // Empty
        LoveHandles {
            allocated: HashMap::new(),
            highest: 0,
            free: Vec::new(),
        }
    }

    /// Make a new handle
    fn make_handle(&mut self, item: FileHandle) -> u64 {
        // Get a number
        let num = self.next_free();

        // Put it in the hashmap.
        // We also assert that we have not already used this number.
        assert!(self.allocated.insert(num, item).is_none(), "We already used this handle number, even though we thought it was free!");

        // All done.
        num
    }

    /// Get the handle back
    fn read_handle(&self, number: u64) -> FileHandle {
        // Handles are not read after freeing, doing so is undefined behavior.
        if let Some(handle) = self.allocated.get(&number) {
            // Cool, it's there.
            handle.clone()
        } else {
            // We are cooked.
            error!("Tried to read a handle that was not allocated!");
            panic!("Use after free on handle.");
        }
    }

    /// Get the next free handle (internal abstraction)
    fn next_free(&mut self) -> u64 {
        // Prefer vec items
        if self.free.is_empty() {
            // Time for a new number then.
            let give = self.highest;
            self.highest += 1;
            return give;
        }

        // There is a vec item.
        self.free.pop().expect("Guarded.")
    }

    /// You need to let go...
    fn release_handle(&mut self, number: u64) {
        // Handles are only ever freed once. Freeing an empty handle is undefined behavior, thus we
        // cant do anything but give up.
        if self.allocated.remove(&number).is_none() {
            // Bad!
            error!("Tried to free a handle that was not allocated!");
            panic!("Double free on handle.");
        };

        // Is this number right below the current highest?
        if number == self.highest - 1 {
            // Yep! Reduce highest.
            self.highest -= 1;
        }
    }
}



lazy_static! {
    static ref LOANED_HANDLES: Arc<Mutex<LoveHandles>> = Arc::new(Mutex::new(LoveHandles::new()));
}





//
// The actual handles
//

use crate::{
    error_types::filesystem::*,
    filesystem::file_handle::file_handle_struct::FileHandle,
    pool::disk::{
        standard_disk::block::{
            directory::directory_struct::{
                DirectoryBlock,
                DirectoryItem
            },
            io::directory::types::NamedItem
        }
    }
};

impl FileHandle {
    /// The name of the file/folder, if it exists.
    /// This will return None on the root.
    pub fn name(&self) -> &str {
        // Get the name, if it exists.
        if let Some(name) = self.path.file_name() {
            name.to_str().expect("Should be valid UTF8")
        } else {
            // No name, this must be the root.
            ""
        }
    }

    /// Allocate the file handle for tracking.
    /// 
    /// Will block.
    /// 
    /// Does not create a new ItemHandle, only stores it.
    pub fn allocate(self) -> u64 {
        // This is blocking.
        let read_handles = &mut LOANED_HANDLES.lock().expect("Other mutex holders should not panic.");
        // Add it
        read_handles.make_handle(self)
    }

    /// Get contents of handle.
    /// 
    /// Will block.
    pub fn read(handle: u64) -> Self {
        // This is blocking
        let read_handles = LOANED_HANDLES.lock().expect("Other mutex holders should not panic.");
        read_handles.read_handle(handle)
    }

    /// Release a handle.
    /// 
    /// Will block.
    pub fn drop_handle(handle: u64) {
        // This is blocking
        let read_handles = &mut LOANED_HANDLES.lock().expect("Other mutex holders should not panic.");
        read_handles.release_handle(handle);
    }

    /// Check if this handle is a file or a directory by attempting to read it from disk, otherwise
    /// deducing the type from it's path string.
    pub fn is_file(&self) -> Result<Option<bool>, c_int> {
        // Annoyingly, rust's PathBuf type doesn't have a way to test if itself is a directory
        // without reading from disk, which makes it completely useless for deducing if the passed argument
        // is a file or folder. Very very annoying.
        //
        // You can't just check for file extensions, since files do not _need_ an extension...
        //
        // The approach i'll take is to see if the path ends with a delimiter. good luck lmao

        // But before we fallback to the path based deduction, we can attempt to load the file from disk if it exists.
        // If it does, we have our sure answer about what type it is, otherwise we will use the crappy string logic.

        // I don't particularly enjoy needing to access the disk here, but chances are if you're trying to find what type
        // something is, you'll be modifying it soon anyways.
        
        let name: String = self.name().to_string();
        debug!("Attempting to deduce if `{}` is a file or directory...", self.path.display());
        
        // Does the parent exist?
        debug!("Checking if it already exists...");
        if let Some(parent) = DirectoryBlock::try_find_directory(self.path.parent())? {
            // Parent does exist, is this item there in either form?
            let file: NamedItem = NamedItem::File(name.clone());
            let directory: NamedItem = NamedItem::Directory(name.clone());
            let maybe_file = parent.find_item(&file)?;
            if maybe_file.is_some() {
                // It was a file
                debug!("Yes, and it's a file.");
                return Ok(Some(true));
            }
            let maybe_directory = parent.find_item(&directory)?;
            if maybe_directory.is_some() {
                // It was a directory.
                debug!("Yes, and it's a directory.");
                return Ok(Some(false));
            }
        }
        debug!("Item did not exist!");

        // Rather than guess, we'll just return that the file did not exist, which should not the be the case for
        // file handles, but maybe the caller just had a stale handle, or is spoofing a new file?
        Ok(None)
    }

    
    /// Loads in and returns the directory item if it exists.
    pub fn get_directory_item(&self) -> Result<DirectoryItem, c_int> {
        // Open the containing folder
        let block = match DirectoryBlock::try_find_directory(self.path.parent())? {
            Some(ok) => ok,
            None => {
                // Containing block did not exist.
                return Err(NO_SUCH_ITEM);
            },
        };

        let named_item = if let Some(bool) = self.get_named_item()? {
            // There was an item with this name.
            bool
        } else {
            // We are trying to deduce the name of an item that does not exist.
            return Err(NO_SUCH_ITEM);
        };

        // Find the item
        if let Some(exists) = block.find_item(&named_item)? {
            // File existed.
            Ok(exists)
        } else {
            // No such item.
            Err(NO_SUCH_ITEM)
        }
    }

    /// Get a named item from this handle.
    pub fn get_named_item(&self) -> Result<Option<NamedItem>, c_int> {
        // Get a name
        let name: String = self.name().to_string();
        let file_check = self.is_file()?;
        
        // If this is none, the caller is trying to extract a named item from
        // an invalid handle or a spoofed file. We return None, since the item did not
        // exist in either form.

        if let Some(is_file) = file_check {
            // An item was there.
            // Deduce the type
            if is_file {
                // yeah its a file
                return Ok(Some(NamedItem::File(name)));
            } else {
                // dir
                return Ok(Some(NamedItem::Directory(name)));
            }
        }

        // There was no item.
        Ok(None)

        
    }
}