use std::{
    ffi::OsStr,
    path::PathBuf
};

use clap::Parser;
use fluster_fs::filesystem::filesystem_struct::{FilesystemOptions, FlusterFS};

// Logging
use env_logger::Env;

#[derive(Parser)]
struct Cli {
    /// Path to the floppy block device.
    #[arg(long)]
    block_device_path: String,
    /// The mount point to mount the Fluster pool.
    #[arg(long)]
    mount_point: String,
    /// Run with virtual floppy disks for testing. Path to put tempfiles in.
    #[arg(long)]
    use_virtual_disks: Option<String>,
    /// Make backups of disks in /var/fluster. Disabling this VERY unsafe, you should
    /// leave this on unless you are doing testing or don't care that much about your data.
    #[arg(long)]
    enable_disk_backup: Option<bool>,
}

fn main() {
    // Start the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("error")).init();

    // Get the block device that the user specifies is their floppy drive
    let cli = Cli::parse();

    // get the mount point
    let mount_point = PathBuf::from(cli.mount_point);

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("Fluster cannot be closed with ctrl+c. You need to unmount the filesystem with `fusermount -u (path)`.");
        println!("Busy? Close everything that may be looking at the filesystem.");
        println!("Still busy? Too bad, wait it out. Or suffer data loss. Your choice.");
        println!("Ignoring...");
    })
    .unwrap();

    // Check if the mount point is valid
    std::fs::create_dir_all(&mount_point).unwrap();

    // Assemble the options
    let use_virtual_disks: Option<PathBuf> = cli.use_virtual_disks.map(PathBuf::from);
    let backup: Option<bool> = cli.enable_disk_backup;

    let options: FilesystemOptions =
        FilesystemOptions::new(use_virtual_disks, cli.block_device_path.into(), backup);

    let filesystem: FlusterFS = FlusterFS::start(&options);

    // Now for the fuse mount options
    let fuse_options = [
        OsStr::new("-onodev"), // Disable dev devices
        OsStr::new("-onoatime"), // No access times
        OsStr::new("-onosuid"), // Ignore file/folder permissions (lol)
        OsStr::new("-orw"), // Read/Write
        OsStr::new("-oexec"), // Files are executable
        OsStr::new("-osync"), // No async.
        OsStr::new("-odirsync"), // No async
        OsStr::new("-oallow_other"), // Allow other users to open the mount point (ie windows outisde of WSL)
        OsStr::new("-ofsname=fluster"), // Set the name of the fuse mount
    ];

    // todo!("fuse mount options for limiting read/write sizes and disabling async");

    // Mount it

    // Internal fuse_mt startup stuff i think, no comments on the function implementation.
    // takes in the filesystem, and the number of threads the filesystem will use
    // Fluster! Is single threaded, so we actually set it to 0 threads. Weirdly.
    let mt_thing = fuse_mt::FuseMT::new(filesystem, 0);


    match fuse_mt::mount(mt_thing, &mount_point, &fuse_options) {
        Ok(_) => {
            // Filesystem was unmounted successfully.
            println!("Fluster! has been unmounted.");
        },
        Err(err) => {
            // rhut row
            println!("Fluster is dead and you killed them. {err:#?}");
        },
    }
}
