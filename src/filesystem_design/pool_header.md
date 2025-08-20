# Pool header

The root disk only holds information about the pool. Blocks cannot be stored to this disk.

| Offset | Length | Field                                                                                          |
| ------ | ------ | ---------------------------------------------------------------------------------------------- |
| 0      | 8      | Magic number for idenifying a fluster drive `Fluster!`                                         |
| 8      | 1      | Bitflags                                                                                       |
| 9      | 2      | Highest known disk number.                                                                     |
| 11     | 2      | Disk with the next free block in the pool.<br />Set to u16::MAX if the final disk has no room. |
| 13     | 4      | Number of blocks free across all disks in the pool.                                            |
| -      | -      | Reserved                                                                                       |
| 148    | 360    | Block usage bitplane                                                                           |
| 509    | 4      | Block CRC                                                                                      |

Bitflags:

| bit | flag                                      |
| --- | ----------------------------------------- |
| 0   | Reserved                                  |
| 1   | Reserved                                  |
| 2   | Reserved                                  |
| 3   | Reserved                                  |
| 4   | Reserved                                  |
| 5   | Reserved                                  |
| 6   | Reserved                                  |
| 7   | Reserved                                  |
| 8   | Marks this as a pool header. Must be set. |
