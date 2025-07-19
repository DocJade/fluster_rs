# Reasons for certain things

# Why 4 byte CRCs?
After 20MB of read-write with random head seeking, I only got 1 failed byte.
A 4 byte crc on our 512 byte block gives us a hamming distance of 6, which is probably even overkill unless
the floppy drive is actively being shaken by a pit bull who mistook it for a toddler.

# Why little endian?
Stack exchange said it was cool.

# What order are the bitflags in the documentation?
flag 0 is the least significant bit

# Why are some reads bigger than they need to be?
I was having an issue reading just 8 bytes into a buffer.
Turns out Windows wont let you read directly from a floppy disk into a buffer smaller than 512 bytes.
This took an annoyingly long time to figure out.

# Why do file extent blocks have a `bytes_free` field even though they arent dynamically allocated?
Ease of use.

# A lot of stuff seems wasteful cpu wise...
Think of it this way, 99% of the time we will be waiting for data from disk, so it evens out!

# Why is an entire disk dedicated to information about the pool?

Chances are, if you are using this filesystem, you are storing many files across many floppies.

Finding a file is a slow and tedious process. We have to start from the first disk and search, possibly swapping between many disks before finding the file we are seeking. Most of this overhead comes from looking up the location of the file inode, not loading the file itself.

Dedicating an entire disk to pool information lets us keep a cache of file locations, skipping the entire search process.
This will result in fewer disk swaps, and a massive speedup in search time.

# Why is the project laid out like that?
Originally, I didn't want to accidentally give access to private functions used for subsystems, but I ended up repeatedly dividing everything up until I was left with Pool::Disk::(Some disk type) then each disk implements its own innards, or uses generic functions from Pool::Disk.

Organizationally, I feel like it makes sense, but the amount of nesting is pretty wild.
This is my first time trying to keep a project organized in a sensible way, so... lol.

# Why are there so many comments?
I've had too many hobby projects in the pass where I've thought to myself, "The code is self documenting". Sure, that might be the case, but the amount of mental effort it takes to understand what's going on 3 days after I wrote something is a LOT higher than if I just left some comments.

I'd prefer too many comments over a headache trying to reverse-engineer what I was thinking previously.

Also it lowers the bar of entry for the casual viewer :D