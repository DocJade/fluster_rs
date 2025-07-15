# Find and allocate blocks

We take in a number (u16) of blocks that the caller wishes to reserve.
We return a `Result<Vec<DiskPointer>, AllocationError>`

`AllocationError` contains types for block situations such as `NotEnoughSpace`, which requires the caller to add more disks to the pool.

### Note:
This operation does not flag blocks as used, this section is read only.
The caller is responsible for updating the allocation tables in the headers of disks they write to.

Finding the next block follows these steps:

## Scanning:
Before we can write any data, we need to ensure we have all the room for it.
This is the discovery phase, no data is written.

Terminology:
- `Start Disk`
- - The disk where the allocation of blocks begins. (Allocations of blocks always go upwards away from the pool disk).
- `Allocation length`
- - The number of blocks we wish to allocate.
- `Note`
- - Copy this value into memory off of the disk for later use.

### Process:
- Insert the pool disk, `Note`: `highest_known_disk`, `pool_blocks_free`, and `disk_with_next_free_block`.
- - If `disk_with_next_free_block` == u16::MAX:
- - - There are no more free blocks. We need another disk. Return `NotEnoughSpace`
- - If `pool_blocks_free` < `Allocation length`:
- - - There aren't enough free blocks. We need another disk. Return `NotEnoughSpace`
- Insert `pool_blocks_free`, hereafter referred to as the `Start disk`
- Goto `Find Blocks`
- If not enough space is found:
- - There aren't enough free blocks in the entire pool.
- - This should have been caught by `pool_blocks_free`.
- - An assertion will go here. We should never hit this branch.
- Update `pool_blocks_free` with how many blocks were allocated
- Update `disk_with_next_free_block`:
- - `Note`: Disk and Block numbers of the final allocated block
- - If the block number is the final block on the disk:
- - - Set `disk_with_next_free_block` to u16::MAX.
- - - Otherwise, set `disk_with_next_free_block` to the Disk of the final allocated block.



## Find Blocks
Incoming arguments:
- `Start Disk`
- - Disk number to start our search from.
- `Allocation length`
- - The number of blocks we wish to allocate.
- `End Disk`
- - The disk number of the final disk in the pool.

Returns:
- `Vec<DiskPointer>`

Terminology:
- Variable `Index`
- - The current disk we are examining. (Lies within range `Start Disk`..=`End Disk`)
- - Starts at `Start Disk`
- Variable `Free blocks seen`
- - Keeps track of how many free blocks we have seen across all disks up to this point.
- Variable `Block pointers`
- - A Vec of `<DiskPointer>`s to each free block we are considering.
- Variable `Blocks remaining`
- - A count of how many more blocks we need to allocate

### Note:
This section does not flag blocks as used, this section is read only.

### Process:
- Insert disk `Index`
- Count number of blocks free in allocation table.
- If the number of blocks free is >= `Blocks remaining`
- - Copy as many disk pointers into `Block pointers` as there are `Blocks remaining`, and return the pointers.
- Copy pointers to all of the free blocks into `Block pointers`, decrement `Blocks remaining` accordingly.
- If `Index` >= `End Disk`
- - There were not enough free blocks.
- - This should not be possible. Caller must guarantee that there is enough free space.
- - Assertion goes here.
- Increment `Index`
- Loop