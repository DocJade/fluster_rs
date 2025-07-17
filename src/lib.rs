// The library/filesystem cannot use unwraps.
#![deny(clippy::unwrap_used)]

// Only use the filesystem in main.rs
pub mod filesystem;