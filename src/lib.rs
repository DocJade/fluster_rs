// The library/filesystem cannot use unwraps.
#![deny(clippy::unwrap_used)]
// Gotta use all the results.
#![deny(unused_results)]
// I need to force some methods to only be used in special places.
// Doing the publicity for it would be a pain, so we just piggyback on
// depreciated
#![deny(deprecated)]

// Only use the filesystem in main.rs
pub mod filesystem;

// Within the crate, we can use:
mod helpers;
mod error_types;
mod pool;
mod tui;