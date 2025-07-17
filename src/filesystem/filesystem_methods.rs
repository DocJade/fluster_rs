// I might blow a fuse.

// At this level of abstraction, we make calls to the Pool type. Nothing lower.

use std::path::PathBuf;

use easy_fuser::{templates::DefaultFuseHandler, FuseHandler};

use crate::{filesystem::filesystem_struct::FilesystemOptions, pool::pool_struct::Pool};

use super::filesystem_struct::FlusterFS;

impl FlusterFS {
    /// Create new filesystem handle, this will kick off the whole process of loading in information about the pool.
    /// Takes in options to configure the new pool.
    pub fn start(options: &FilesystemOptions) -> Self {
        // Right now we dont use the options for anything, but they do initalize the globals we need, so we still need to pass it in.
        #[allow(dead_code)]
        #[allow(unused_variables)]
        let unused = options;
        FlusterFS {
            inner: Box::new(DefaultFuseHandler::new()),
            pool_info: Pool::load(),
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

}