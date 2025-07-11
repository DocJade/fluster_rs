# Block layout
512 bytes in size
A floppy disk can hold 2880 blocks of this size.


# Disk header format
The disk header lives on block 0 of every disk.

The following is in order.
8 bytes: Magic number for identifying a fluster drive. `Fluster!`
1 byte: bitflags
    0: This is a dense disk
    1: Reserved for future use
    2: Reserved for future use
    3: Reserved for future use
    4: Reserved for future use
    5: Reserved for future use
    6: Reserved for future use
    7: Reserved for future use
2 bytes: Disk number 
138 bytes: Reserved
360 bytes: Block usage bitplane

Final 4 byte: crc