# Block layout

512 bytes in size
A floppy disk can hold 2880 blocks of this size.

# Header update situations:

New disk is created:

- Highest known disk has to be updated on the root disk (disk 0)

# Disk header format

The disk header lives on block 0 of every disk.

Header format:

| offset | length | Field                                                 |
| ------ | ------ | ----------------------------------------------------- |
| 0      | 8      | Magic number for identifying a fluster drive.Fluster! |
| 8      | 1      | Bitflags                                              |
| 9      | 2      | Disk number (u16)                                     |
| -      | -      | Reserved                                              |
| 148    | 360    | Block usage bitplane                                  |
| 509    | 4      | CRC                                                   |

Bitflags:

| bit | flag                                          |
| --- | --------------------------------------------- |
| 0   | Reserved                                      |
| 1   | Reserved                                      |
| 2   | Reserved                                      |
| 3   | Reserved                                      |
| 4   | Reserved                                      |
| 5   | Reserved                                      |
| 6   | Marker bit, Must always be set.               |
| 7   | Reserved for Dense disks.  Must never be set. |
| 8   | Reserved for Pool headers. Must never be set. |

8 bytes:
1 byte: bitflags
2 bytes: Disk number
138 bytes: Reserved
360 bytes: Block usage bitplane

Final 4 byte: crc
