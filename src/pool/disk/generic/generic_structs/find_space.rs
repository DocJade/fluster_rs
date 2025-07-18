// A cool function that finds free space in a slice of bytes

// Trait constraint that all input types must meet
pub trait BytePingPong {
    /// Converts a type to its byte representation
    fn to_bytes(&self) -> Vec<u8>;
    /// Converts bytes into itself, and
    /// will discard extra trailing bytes.
    fn from_bytes(bytes: &[u8]) -> Self;
}

/// Find a contiguous space of x bytes in the input slice.
/// Returns an index into the input data where the next `requested_space` bytes are empty.
/// We assume the caller already checked if there is enough room, so if we do not find
/// a space big enough, we will return None.
pub fn find_free_space<T: BytePingPong>(data: &[u8], requested_space: usize) -> Option<usize> {
    // Assumptions:
    // All incoming types will have bitflags
    // - All incoming bitflags will have a marker bit in position 7
    // Free space will be all 0's

    // We will:
    // - Look at a byte and check for the marker bit
    // - - If there is no bit, this must be the start of unused space
    // - - If there is a bit, get the length of this item, and jump ahead that far, start over
    // - Check if the next `requested_space` bytes are empty:
    // - - Yes? Return the current index
    // - - No? Find which byte had data, and set the index to that byte. Start over.

    // How far we are indexed into the data
    let mut index: usize = 0;

    // Sanity check, are we requesting more bytes than there is room possibly for bytes?
    assert!(
        requested_space <= data.len(),
        "We shouldn't try to find `x` bytes free space in a slice smaller than `x`."
    );

    // We wont search bytes that are too far into the block to have enough space after them
    // for the incoming data.
    while index <= data.len() - requested_space {
        // Check for the marker bit
        if data[index] & 0b10000000 != 0 {
            // The bit is set. We need to seek forwards.

            // To find out how far we need to seek, we will convert the bytes starting at offset
            // to type <T>, then convert that type back to bytes again, and get the length of that
            // This might be tad wasteful, but it is simple lol.

            // Don't like it? Make a pull request! :D

            index += T::from_bytes(&data[index..]).to_bytes().len();
            continue;
        }

        // The bit is not set. Check if there's room
        let enough_space: bool = data[index..index + requested_space]
            .iter()
            .all(|&byte| byte == 0);

        if enough_space {
            // Found space!
            return Some(index);
        }

        // There was a byte in the way, find which byte caused it
        let non_empty_byte_offset: usize = data[index..index + requested_space]
            .iter()
            .position(|&byte| byte != 0)
            .expect("There has to be a byte in the way.");

        // Move that far forward, then try again.
        // The index we are already on MUST be either zero, or the start of a <T>
        // Since we already know we arent at the start of <T>, we will always jump at least
        // one byte forwards.
        index += non_empty_byte_offset;
        continue;
    }

    // If we made it out of the while loop, that must mean there is not an open space.
    return None;
}
