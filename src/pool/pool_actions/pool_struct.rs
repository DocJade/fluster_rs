// Did you know, if lightning struct a pool, everyone dies?
// Imports

use crate::pool::disk::pool_disk::block::header::header_struct::PoolDiskHeader;

use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

// The global access to the pool.
// It was either have a globally accessible pool, or put a reference to the pool in every method... No thanks.
// Know a cleaner way? Make a pull request :D

// This is done with a OnceCell so I dont have to spoof a fake pool into here before actually loading one up.

lazy_static! {
    pub(crate) static ref GLOBAL_POOL: OnceCell<Arc<Mutex<Pool>>> = OnceCell::new();
}

// Structs, Enums, Flags

// All of the information we need about a pool to do our job.
#[derive(Debug)]
pub struct Pool {
    pub(crate) header: PoolDiskHeader,
}