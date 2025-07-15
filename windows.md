# How to use fluster on windows

You cannot build fluster on windows due to the `fuser` crate linking to unix stuff in libc, even if you install a compatibility library like WinFsp. Trust me, I tried.

You need to build/run from inside of WSL.
Required dependancies (non exhaustive):
- build-essential
- libfuse3-dev
- rust (duh)