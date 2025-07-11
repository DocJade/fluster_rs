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