# Reasons for certain things

# Why 4 byte CRCs?
After 20MB of read-write with random head seeking, I only got 1 failed byte.
A 4 byte crc on our 512 byte block gives us a hamming distance of 6, which is probably even overkill unless
the floppy drive is actively being shaken by a pit bull who mistook it for a toddler.

# Why little endian?
Stack exchange said it was cool.