# Dense disk

Sometimes, you've got a really big file. And I mean REALLY big.

If a file is over the size of a full floppy, find out how many full floppies it can span and reserve those
the rest of the file will go in data blocks as usual

# Disk header format

The disk header lives on block 0 of every disk.

Header format:

| offset | length | Field                                                 |
| ------ | ------ | ----------------------------------------------------- |
| 0      | 8      | Magic number for identifying a fluster drive.Fluster! |
| 8      | 1      | Bitflags                                              |
| 9      | 2      | Disk number                                           |
| -      | -      | Reserved                                              |
| 148    | 360    | Block usage bitplane                                  |
| 509    | 4      | CRC                                                   |

Bitflags:

| bit | flag                                           |
| --- | ---------------------------------------------- |
| 0   | Reserved                                       |
| 1   | Reserved                                       |
| 2   | Reserved                                       |
| 3   | Reserved                                       |
| 4   | Reserved                                       |
| 5   | Reserved                                       |
| 7   | Reserved for Standard disks. Must never be set.|
| 6   | Marker bit, Must always be set.                |
| 8   | Reserved for Pool headers. Must never be set.  |