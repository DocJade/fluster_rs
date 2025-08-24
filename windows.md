This is my punishment for not using Linux in the year of the Linux desktop smh.
Special thanks to [Chris Harringon](https://chris.harrington.mn/)!

# How to use fluster on windows

You cannot build fluster on windows due to the `fuser` crate linking to unix stuff in libc, even if you install a compatibility library like WinFsp. Trust me, I tried.

You need to build/run from inside of WSL.

# Step 0: Usb storage module
The default WSL kernels do not have the usb_storage module enabled, we will need to build our own.

Follow [this](https://chris.harrington.mn/project/2022/07/30/wsl2-usb-storage.html) up until the `Windows USB-IP` section.
`sudo modprobe usb-storage` should not fail. You can check if it was actually loaded with `lsmod`.

# Step 1: USB floppy passthrough.

You will also need to pass through the floppy drive to WSL.
This will not persist through a reboot, or restarting WSL.

You cannot mount a floppy drive normally with `wsl --mount`, so we will have to pass through the USB device.
This obviously assumes you have a USB floppy drive. If you have an internal drive you want to use, I cannot help you. Good luck!

On windows you will need the `usbipd-win` package
`winget install --interactive --exact dorssel.usbipd-win`

Then find your drive with `usbipd list`

For reference, my drive shows up like so:
```
> usbipd list

Connected:
BUSID  VID:PID    DEVICE                                                        STATE
...
6-11   0644:0000  TEAC USB Floppy                                               Not shared
```

Now we will bind the floppy to WSL using the bus id from the above command. (This requires an elevated command prompt / powershell window)
```
usbipd bind --busid X-X
```
There will be no output.

Now we attatch it to WSL
```
usbipd attach --wsl --busid X-X
```
You will see some information about it finding your WSL distribution, and probably hear a USB discconect sound.
The floppy drive will also (probably) spin up, assuming you have a disk inserted.

Double check that the floppy is there
```
NAME
    MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS
$ lsblk
sde   8:64   1   1.4M  0 disk
```

Take note of which `sd` device it is. Mine happens to be `sde`.
This will be the path that you use when starting Fluster.

***A grain of salt***
This is SO unstable, you might also have to disable, then uninstall the floppy drive in device manager first.
If your disk is spinning forever without seeming to do anything, you need to wait it out. Pulling out the disk
(at least on my drive) in that state makes USBIPD give up and unmount the drive.

### Note:
My floppy disks were completely blank (every byte had been zeroed out), if your disks arent completely blank, im unsure if this will effect mounting the drive.
If your floppy already has a mountpoint, unmount it before continuing.

# Step 2.5:
You need to allow `other user` mounting in fuse to let you access Fluster! from windows through WSL.
`sudo nano /etc/fuse.conf`
Uncomment `#user_allow_other`



# Step 3: Building and running: 
Required dependancies (non exhaustive):
- build-essential
- libfuse3-dev
- rust (duh)

Open the Fluster! source code directory (the one that contains cargo.toml).
Build with `cargo build --release`
(You can also build with `--profile floppy` for a smaller binary. Using `upx --best` on it should make it fit on a floppy if you
really wanna have fun with it. Currently shrinks to under 800kb!)

Pick a folder to mount to, for this example I will mount in `~/mounted/`.
Remember your floppy drive path (The SD one). For this example, my drive is at `sde`.

### YOU BETTER MAKE DAMN SURE YOU'RE PASSING IN THE FLOPPY DRIVE

### READ THIS
Fluster! WILL overwrite data on whatever block device you pass it.
If you do not know what "block device" even means, this is your final warning. Do NOT continue further.

Since we will be writing directly to the floppy disk without a pre-existing filesystem (we are the file system!) we
unfortunately need to escalate permissions.

So we'll just run it at sudo. Great idea I know.

There is probably a safer way to do this (Such as using udev rules), but I got bored reading the documentation.


Run fluster:
- Go to the output directory for the binary you built (./target/release/)

Run it as sudo with:
`sudo ./fluster_fs --block-device-path "/dev/sdX" --mount-point "/home/(your username here)/mounted/fluster"`

If you don't want to run as sudo/root, you're smart.
Smart enough to figure out another solution. Good luck!

Be warned, this will consume the terminal window you started the program with.
Try using `tmux` to split your window if needed. (Must be installed ofc, Google it.)

You can also run with debug info by pre-pending `RUST_LOG=debug` to the command if you're a nerd.
This does add some performance overhead, which would only really matter if you're using the
secret `--use-virtual-disks` option.