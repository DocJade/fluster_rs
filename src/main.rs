use std::{
    ffi::OsStr, path::PathBuf, process::exit, sync::{
        atomic::{AtomicBool, Ordering}, Arc
    }
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
}

fn main() {
    // Start the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Get the block device that the user specifies is their floppy drive
    let cli = Cli::parse();

    // get the mount point
    let mount_point = PathBuf::from(cli.mount_point);

    // Functions from easy_fuser/examples/zip_fs/src/main.rs

    // Set up the cleanup function
    let once_flag = Arc::new(AtomicBool::new(false));
    let cleanup = |mount_point: &PathBuf, once_flag: &Arc<AtomicBool>| {
        if once_flag.clone().swap(true, Ordering::SeqCst) {
            return;
        }
        println!("Unmounting filesystem...");
        let _ = std::process::Command::new("fusermount")
            .arg("-u")
            .arg(mount_point)
            .status();
    };

    // Set up Ctrl+C handler
    let mount_point_ctrlc = mount_point.clone();
    let onceflag_ctrlc = once_flag.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C, unmounting...");
        cleanup(&mount_point_ctrlc, &onceflag_ctrlc);
        exit(1);
    })
    .unwrap();

    // Check if the mount point is valid
    std::fs::create_dir_all(&mount_point).unwrap();

    // Assemble the options
    let use_virtual_disks: Option<PathBuf> = cli.use_virtual_disks.map(PathBuf::from);

    let options: FilesystemOptions =
        FilesystemOptions::new(use_virtual_disks, cli.block_device_path.into());

    let filesystem: FlusterFS = FlusterFS::start(&options);

    // Now for the fuse mount options
    let fuse_options = [
        OsStr::new("nodev"), // Disable dev devices
        OsStr::new("noatime"), // No access times
        OsStr::new("nosuid"), // Ignore file/folder permissions (lol)
        OsStr::new("rw"), // Read/Write
        OsStr::new("exec"), // Files are executable
        OsStr::new("sync"), // No async.
        OsStr::new("dirsync"), // No async
        OsStr::new("fsname=fluster"), // Set the name of the fuse mount
    ];

    // todo!("fuse mount options for limiting read/write sizes and disabling async");

    // Mount it

    // Internal fuse_mt startup stuff i think, no comments on the function implementation.
    // takes in the filesystem, and the number of threads the filesystem will use
    // Fluster! Is single threaded.
    let mt_thing = fuse_mt::FuseMT::new(filesystem, 1);


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
