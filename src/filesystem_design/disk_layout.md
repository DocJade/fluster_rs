# Disk Layout
Block 0: Disk header
Block 1: Inode block

Remaining blocks: any inode, directory, or data.


# Block types
Header (See `disk_header`)
Inode
Directory Data
File Extents
Data

# Data block
1 byte: bitflags
    0: Reserved for future use
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use

remaining bytes: raw data

final 4 bytes: CRC

# Directory block
1 byte: bitflags
    0: Reserved for future use
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
2 bytes: number of free bytes
4 bytes: next Inode block
    - 2 Bytes: Disk number
    - 2 Bytes: Block on disk
    - if this is u16::MAX then we are at the end of the block chain

remaining bytes: directory data

final 4 bytes: CRC

# File Extents block
1 byte: bitflags
    0: Reserved for future use
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
4 bytes: Next block
    - 2 Bytes: Disk number
    - 2 Bytes: Block on disk
    - if all 4 bytes are full 1's, this is the final block

remaining bytes: extent data

final 4 bytes: CRC

# Inode block
1 byte: bitflags
    0: This is the last inode on the disk.
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
2 bytes: number of free bytes
2 bytes: next Inode block (Either a disk number, or a block number depending on flags.)
    - If u16:MAX then this is the end of the inode chain

remaining bytes: inode data

final 4 bytes: CRC

If you are on the final inode disk and realize you need to make another inode block, you update the
bitflag and reserve another block.
If you are out of blocks on that disk, go to the next disk if bit 1 is set.
If bit 1 is not set, then you can simply go to the next disk indicated. Otherwise you must find a _NEW_ disk
to put the next inode block on and update flags accordingly. (New disk inodes must be in the default position)

On disk 0, the first inode in the block MUST be a directory referencing `/` aka the root.