// This is where the fun begins
// This is generic between platforms.

use easy_fuser::templates::DefaultFuseHandler;

pub struct FlusterFS {
    pub(super) inner: Box<DefaultFuseHandler>,
}