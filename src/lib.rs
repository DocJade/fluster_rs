// The library/filesystem cannot use unwraps.
#![deny(clippy::unwrap_used)]

// Asserts need to have a reason.
#![deny(clippy::missing_assert_message)]

// Gotta use all the results.
#![deny(unused_results)]
// I need to force some methods to only be used in special places.
// Doing the publicity for it would be a pain, so we just piggyback on
// depreciated
#![deny(deprecated)]

// Only use the filesystem in main.rs
// We only support 64 bit systems. since we expect usize to be that size.
#[cfg(target_pointer_width = "64")]
pub mod filesystem;

// Within the crate, we can use:
mod helpers;
mod error_types;
mod pool;
pub mod tui;