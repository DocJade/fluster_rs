// Errors for header conversions.
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
/// Errors related to block manipulation. Not disk level modification, but our custom block types.
pub enum HeaderError {
    #[error("This is not a header of the requested type.")]
    Invalid,
    #[error("The block that was requested to turn into a header is completely blank.")]
    Blank,
}