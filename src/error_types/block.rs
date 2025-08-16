// Blocks usually return similar types of errors.
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
/// Errors related to block manipulation. Not disk level modification, but our custom block types.
pub enum BlockManipulationError {
    #[error("Adding content to this block failed, due to the block not having enough capacity for the new content.")]
    OutOfRoom,
    #[error("This method can only be called on the final block in a chain of this type of block.")]
    NotFinalBlockInChain,
    #[error("The arguments given for this operation are out of bounds, or otherwise not supported.")]
    Impossible,
    #[error("The data that was attempted to be retrieved from this block did not exist.")]
    NotPresent
}