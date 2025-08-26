  
<h1 align="center">
  <br>
  <img src="https://github.com/DocJade/fluster_rs/blob/master/img/flustered.png?raw=true" alt="Fluster" width="256">
  <br>
  Fluster
  <br>
</h1>

<h3 align="center">A futuristic filesystem that's stuck in the past.</h4>

<p align="center">
  <img alt="Blazingly fast!" src="https://img.shields.io/badge/Blazingly_fast!-000000?logo=rust&logoColor=white">
  <a href="https://kofi.docjade.com/">
    <img alt="Support me on Ko-fi!" src="https://img.shields.io/badge/Support%20me%20on%20Ko--fi!-FF5E5B?logo=ko-fi&logoColor=white">
  </a>
  <a href="https://en.wikipedia.org/wiki/Gluten">
	<img alt="Gluten free!" src="https://img.shields.io/badge/Gluten_free!-blue">
</a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#how-to-use">How To Use</a> •
  <a href="#credits">Credits</a> •
  <a href="#license">License</a>
</p>


<p align="center">
	PLACEHOLDER IMAGE
	<img src="https://ralfvanveen.com/wp-content/uploads/2021/06/Placeholder-_-Glossary.svg" alt="Fluster" width="256">
</p>

## Features

*  Multi-disk
	* Spans a single filesystem across as many floppy disks as are required to store the data, treating them as a single pool.
* Disk failure detection
	* Automatically detects and troubleshoot drive and disk issues.
* Automatic backups
	* Floppy disks are unreliable, so every block operation is backed up to `/var/fluster` in case disk recovery is required.
* Tiered caching
	* Triple tiered, in-memory cache to minimize disk swapping, while only using 2 floppy disks worth of memory.
* Error checking
	* Every 512 byte block has a 4 byte CRC to detect corruption or bad reads, and disk operations will automatically retry if the CRC fails.
* FUSE based
	* Built on [FUSE](https://github.com/libfuse/libfuse), which makes Fluster! mountable on any UNIX or UNIX-like system that supports FUSE.

 
## How To Use

To clone and run Fluster!, you'll need [Rust](www.rust-lang.org), a FUSE implementation, a floppy drive, and at least two floppy disks.

### Prerequisites
#### For Linux & macOS

- **Install:**
	- **Rust:** Follow the official installation [guide](https://www.rust-lang.org/).
	- **FUSE:** On most Linux distributions, libfuse is available through your package manager (e.g., sudo apt-get install libfuse-dev).
		- On macOS, you may need [macFUSE](https://macfuse.github.io/), although I have not tested Fluster! on MacOS at all, since you should use a real operating system.

#### For Windows Users
- If you're running Fluster! on Windows, please [read this guide](https://github.com/DocJade/fluster_rs/blob/master/windows.md).

### Building and running

#### Build Fluster!:
```bash
# Clone the repository
git clone https://github.com/DocJade/fluster_rs

# Go into the repository
cd fluster_fs

# Build with the recommended 'floppy' profile
cargo build --profile floppy
```

#### Run Fluster!:
```bash
# Example usage:
# Create a directory to mount the filesystem
mkdir ~/fluster_mount_point
# Run the app (requires root privileges for mounting)
sudo ./target/floppy/fluster_fs --block-device-path "/dev/sdX" --mount-point "~/fluster_mount_point"
```
- Replace /dev/sdX with the actual path to your floppy drive.

#### Unmounting Fluster!:
```bash
fusermount -u ~/fluster_mount_point
```
- Do note that unmounting Fluster! does not immediately shut down fluster, you will still need to swap disks to flush the cache to disk.
## Credits

- [DocJade](https://docjade.com/) (That's me!)
- [Rust](https://www.rust-lang.org/) ([Not the bad Rust](https://rust.facepunch.com/))
- [The Rippingtons](https://www.rippingtons.com/), who kept me from going insane while writing this.
- [Femtanyl](https://femtanyl.bandcamp.com/), who helped me go insane while writing this.

## See Fluster! in action

[YouTube link to fluster factorio run will go here.](https://example.com/)

## You may also like...

- Pornography, search "boobs" on google for more info.

## License

TBD

---

> [I should just come up with a cool name and the rest will like as itself.](https://www.youtube.com/watch?v=dmzk5y_mrlg)
