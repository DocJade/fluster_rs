use bitflags::bitflags;
use libc::c_int;
use log::warn;

//
//
// ======
// Flag type
// ======
//
//

// Flags are handled with bare u32 integers,
// hence we have a bitflag type to make dealing with them easier.

// Open documentation:
// https://man7.org/linux/man-pages/man2/openat.2.html
// The flags are in libc::

// When it says "Has no effect", I mean on the fluster side. Fluster just does not care
// about this flag being set or unset.

// I'm pretty sure that the read/write flags do not overlap. If they do I will split this into multiple types.

// All of the c flags are i32 for reasons unknown to me, so we have to cast all of them lol
// Not sure why the fuse_mt crate uses u32...

bitflags! {
    /// Flags that items have.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) struct ItemFlag: u32 {
        /// The file is opened in append mode.
        /// Before each write, the file offset is positioned at the end of the file.
        /// The modification of the file offset and write is done as one atomic step.
        const APPEND = libc::O_APPEND as u32;
        
        /// Async, fluster does not support this. Thus we will not
        /// add this bit to the flags.
        /// 
        /// Has no effect.
        const O_ASYNC = libc::O_ASYNC as u32;

        /// Has to do with closing when executing, ignoring, good luck.
        /// 
        /// Has no effect
        const O_CLOEXEC = libc::O_CLOEXEC as u32;

        /// If the path does not exist, create it as a regular file.
        const CREATE = libc::O_CREAT as u32;

        /// Has to do with direct IO. We don't really care, since we have no special
        /// handling for this kinda thing.
        /// 
        /// Has no effect.
        const O_DIRECT = libc::O_DIRECT as u32;

        /// Fail the open if the path is not a directory.
        const ASSERT_DIRECTORY = libc::O_DIRECTORY as u32;

        /// Has to do with data syncing. We do not care.
        /// 
        /// Has no effect
        const O_DSYNC = libc::O_DSYNC as u32;

        /// Ensure that call creates the file. if this is set and O_CREAT is also set, we're
        /// supposed to turn a EEXIST on open if path already exists.
        /// 
        /// O_EXCL is undefined if used without O_CREAT (unless pointing at block devices which fluster is not.)
        const CREATE_EXCLUSIVE = libc::O_EXCL as u32;
        
        /// Deals with filesizes with offsets that can be greated than off_t (I think that's 32 bit)
        /// 
        /// If you need files that big, fluster is not the tool for you.
        /// Has no effect.
        const O_LARGEFILE = libc::O_LARGEFILE as u32;
        
        /// Do not update file access time.
        /// 
        /// Cool, we don't support that anyways.
        /// 
        /// Has no effect.
        const O_NOATIME = libc::O_NOATIME as u32;

        /// If path is a terminal device, do not control it or whatever.
        /// 
        /// Fluster! does not have terminal devices.
        /// Has no effect.
        const O_NOCTTY = libc::O_NOCTTY as u32;

        /// Symbolic link related. We do not support links.
        // const O_NOFOLLOW = libc::O_NOFOLLOW;
        
        /// Open in non-blocking mode.
        /// Fluster is single threaded. EVERYTHING blocks dawg.
        /// 
        /// Has no effect.
        const O_NONBLOCK = libc::O_NONBLOCK as u32;
        const O_NDELAY = libc::O_NDELAY as u32; // Alternate name for same flag

        /// Dont follow symlinks.
        /// 
        /// Fluster does not have symlinks.
        /// Has no effect.
        const O_NOFOLLOW = libc::O_NOFOLLOW as u32; // Alternate name for same flag
        
        /// Gets file descriptor for this path but not the actual file.
        /// 
        /// Guess what buddy? you'll just get the whole file regardless.
        /// 
        /// Has no effect.
        const O_PATH = libc::O_PATH as u32;
        
        /// Do synchronized file I/O.
        /// 
        /// This is supposed to force sync to disk, but we are silly and don't care :)
        /// 
        /// Has no effect.
        const O_SYNC = libc::O_SYNC as u32;
        
        /// Creates unnamed tempfiles.
        /// 
        /// We do not support this.
        /// Has no effect.
        const O_TMPFILE  = libc::O_TMPFILE as u32;
        
        /// If the file already exists, truncate it.
        /// 
        /// There is already a truncate method on the filesystem, but this may get called elsewhere
        /// so we still need to care elsewhere.
        const TRUNCATE = libc::O_TRUNC as u32;


    }
}

/// Convert a flag to a u32 for use in returning.
impl From<ItemFlag> for u32 {
    #[inline]
    fn from(value: ItemFlag) -> Self {
        value.bits()
    }
}

/// Tried to convert a u32 into a valid flag, returns an `Unsupported` error if a non-existent flag is set.
impl ItemFlag {
    pub fn deduce_flag(value: u32) -> Result<Self, c_int> {
        // All bits must be used. We need to know what they all are.
        if let Some(valid) = ItemFlag::from_bits(value) {
            // All good.
            Ok(valid)
        } else {
            // Has invalid bits set.
            // We will print some information to deduce the unused bits.
            warn!("Incoming flag bits had unused bits set. This operation is unimplemented.");
            warn!("Listing known and unknown flags:");

            warn!("Known:");
            let known = ItemFlag::from_bits_truncate(value);
            for name in known.iter_names() {
                warn!("`{}` with value `{}` (binary: `{:0>32b}`)", name.0, name.1.bits(), name.1.bits())
            }

            warn!("Unknown:");
            let unknown_bits = value & !ItemFlag::all().bits();
            warn!("{unknown_bits:0>32b}");
            // now print out the values of those bits
            warn!("Values for those bits:");
            for i in 0..32 {
                // shift over to mask out the bit
                let mask: u32 = 1 << i as u32;
                // if the bit is set, print the value
                if mask & unknown_bits != 0 {
                    warn!("{mask} (hex {mask:X})")
                }
            }

            // I've spent several hours trying to track down what 0x8000 is supposed to be a flag for, no luck.
            // I'm just going to assume its some internal flag at this point. We will ignore all flag bits that we dont know.

            warn!("Continuing anyways, but ignoring those bits may have side effects.");
            Ok(known)
        }
    }
}