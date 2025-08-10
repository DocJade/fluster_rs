This is my punishment for not using Linux in the year of the Linux desktop smh.
Special thanks to [Chris Harringon](https://chris.harrington.mn/)!

# How to use fluster on windows

You cannot build fluster on windows due to the `fuser` crate linking to unix stuff in libc, even if you install a compatibility library like WinFsp. Trust me, I tried.

You need to build/run from inside of WSL.

# Step 0: Usb storage module
The default WSL kernels do not have the usb_storage module enabled, we will need to build our own.

Follow [this](https://chris.harrington.mn/project/2022/07/30/wsl2-usb-storage.html) up until the `Windows USB-IP` section.
`sudo modprobe usb-storage` should not fail. You can check if it was actually loaded with `lsmod`

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

todo