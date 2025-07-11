# Example Traversal

Lets find `/foo/bar.txt`

### Start at Root Inode
The location of the root inode is fixed (Disk 0, Block 1, Slot 0).
Read the Inode at this address. It's a Directory type.

### Find Root's Directory Data
From the root Inode, get the pointer to its Directory block.

### Scan Root's Directory Data for `foo`
Read that DirectoryDataBlock.
Search its children map for the key `foo`.
If found, you get the InodeAddress for `foo`.
If not found and there's a next_block pointer, follow the chain and repeat the search.

### Find `foo`'s Directory Data
Read the Inode at `foo`'s address. It's also a Directory type.
From this Inode, get the pointer to its Directory block.

### Scan `foo`'s Directory Data for `bar.txt`
Read the DirectoryDataBlock for foo.
Search its children map for the key `bar.txt`.
If found, you get the InodeAddress for `bar.txt`.

### Find `bar.txt`'s Extents
Read the Inode at `bar.txt`'s address. It's a File type.
From this Inode, get the pointer to its first_extent_block.

### Read File Data
Read that FileExtentBlock to get the list of FileExtents,
which finally tell you which blocks on which disks hold the actual file data.
Extents in this file are in order.


# Inode format
1 byte: bitflags
    - 0: File type (0 directory, 1 file)
    - 1: Reserved for future use
    - 2: Reserved for future use
    - 3: Reserved for future use
    - 4: Reserved for future use
    - 5: Reserved for future use
    - 6: Reserved for future use
    - 7: Reserved for future use
6-12 bytes: Inode data
    * File:
        - 8 bytes for size
        - 4 bytes for pointer to the File Extents block
            - 2 Bytes: Disk number
            - 2 Bytes: Block on disk
    * Directory:
        - 4 bytes for pointer to Directory Data block
            - 2 Bytes: Disk number
            - 2 Bytes: Block on disk
12 bytes: Created timestamp
    - 8 bytes: Seconds since epoch
    - 4 bytes: nanosecond offset
12 bytes: Modified timestamp
    - 8 bytes: Seconds since epoch
    - 4 bytes: nanosecond offset


# Inode block
see `disk_layout`

# Directory block
see `disk_layout`


# Directory item format
1 byte: bitflags
    0: Inode is on this disk
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
1 byte: length of item name
? bytes: item name
3-5 bytes: inode location
    - 2 Bytes: Disk number (Not included if flag set)
    - 2 Bytes: Block on disk
    - 1 Byte: Index into inode block



# File Extents block
see `disk_layout`

# Extent format

1 byte: bitflags
    0: This extent is a dense-disk
    1: The block is on this disk
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
4-6 Bytes: extent information
    - 2 Bytes: Disk number (Not included if block is local)
    - 2 Bytes: Start block (Not included if dense)
    - 2 Bytes: Length (Not included if dense)
