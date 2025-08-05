// Make the handle do things.

use std::{collections::HashMap, sync::{Arc, Mutex}};

use lazy_static::lazy_static;

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
        todo!()
    }

    /// Get the handle back
    fn read_handle(&self, number: u64) -> FileHandle {
        // Handles are not read after freeing, doing so is undefined behavior.
        todo!()
    }

    /// Get the next free handle (internal abstraction)
    fn next_free(&self) -> u64 {
        todo!()
    }

    /// You need to let go...
    fn release_handle(&mut self, number: u64) {
        // Handles are only ever freed once. Freeing an empty handle is undefined behavior, thus we
        // cant do anything but give up.
        todo!()
    }
}



lazy_static! {
    static ref LOANED_HANDLES: Arc<Mutex<LoveHandles>> = Arc::new(Mutex::new(LoveHandles::new()));
}





//
// The actual handles
//

use crate::{filesystem::file_handle::file_handle_struct::FileHandle, pool::disk::{drive_struct::FloppyDriveError, standard_disk::block::directory::directory_struct::DirectoryItem}};

impl FileHandle {
    /// The name of the file/folder, if it exists.
    /// This will return None on the root.
    pub fn name(&self) -> Option<&str> {
        // Get the name, if it exists.
        if let Some(name) = self.path.file_name() {
            Some(name.to_str().expect("Should be valid UTF8"))
        } else {
            // No name, this must be the root.
            Some("")
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

    // Check if this handle is a file or a directory.
    pub fn is_file(&self) -> bool {
        // Annoyingly, rust's PathBuf type doesn't have a way to test if itself is a directory
        // without reading from disk, which makes it completely useless for deducing if the passed argument
        // is a file or folder. Very very annoying.
        //
        // You can't just check for file extensions, since files do not _need_ an extension...
        //
        // The approach i'll take is to see if the path ends with a delimiter. good luck lmao

        // The delimiter is platform specific too.
        static DELIMITER: char = std::path::MAIN_SEPARATOR;

        // If the path is empty, its the root node, which is a directory
        if self.path.iter().count() == 0 {
            // This is the root
            return false;
        }
        
        // Check if it ends with the delimiter.
        self.path.as_os_str().to_str().expect("Should be valid utf8").ends_with(DELIMITER)
    }

    
    /// Loads in and returns the directory item. Assuming it exists.
    pub fn get_directory_item(&self) -> Result<DirectoryItem, FloppyDriveError> {
        todo!()
    }
}