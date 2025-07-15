# Pool header

The root disk only holds information about the pool. Blocks cannot be stored to this disk.
This raises the requirement to having a minimum of 2 disks, but the overhead should pay itself off in read speed via caching.
// TODO: Add caching.

| Offset | Length | Field                                                                                          |
| ------ | ------ | ---------------------------------------------------------------------------------------------- |
| 0      | 8      | Magic number for idenifying a fluster drive `Fluster!`                                       |
| 8      | 1      | Bitflags                                                                                       |
| 9      | 2      | Highest known disk number.                                                                     |
| 11     | 2      | Disk with the next free block in the pool.<br />Set to u16::MAX if the final disk has no room. |
| 13     | 2      | Number of blocks free across all disks in the pool.                                            |
| -      | 509    | Reserved                                                                                       |
|        |        |                                                                                                |
| 509    | 4      | Block CRC                                                                                      |
