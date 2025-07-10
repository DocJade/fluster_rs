# Speed improvement ideas

# Disk pre-seek
If we know we are about to change disks, is it possible to pre-align the head of the drive
to the next block we will read on the next disk while the user swaps?

# In-memory inode cache
This might be practically mandatory to get any usability out of the file system unless
we want to be swapping tons of disks for every read operation.

You should be able to enable or disable this on the fly if you want.