// I might blow a fuse.

use std::path::PathBuf;

use easy_fuser::{templates::DefaultFuseHandler, FuseHandler};

use super::filesystem_struct::FlusterFS;

// Ease of use
impl FlusterFS {
    /// Create new filesystem handle
    pub fn new() -> Self {
        FlusterFS { inner: Box::new(DefaultFuseHandler::new()) }
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
}