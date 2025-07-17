// The library/filesystem cannot use unwraps.
#![deny(clippy::unwrap_used)]

// Only use the filesystem in main.rs
pub mod filesystem;

// Imports for the rest of the crate
pub(crate) mod pool;
pub(crate) mod helpers;