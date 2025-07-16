use std::{fs::File, path::{Path, PathBuf}, process::exit, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};

use fluster_fs::filesystem::filesystem_struct::FlusterFS;
use clap::Parser;
use lazy_static::lazy_static;

// Global varibles
// We need to access the path quite deep down into the disk functions, passing it all the way down there would be silly.
// Same with the virtual disk flag.
lazy_static! {
    static ref USE_VIRTUAL_DISKS:  Mutex<bool> = Mutex::new(false);
    static ref FLOPPY_PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

#[derive(Parser)]
struct Cli {
    /// Path to the floppy block device.
    #[arg(long)]
    block_device_path: String,
    /// The mount point to mount the Fluster pool.
    #[arg(long)]
    mount_point: String,
    /// Run with virtual floppy disks for testing.
    #[arg(long)]
    use_virtual_disks: Option<bool>
}

fn main() {
    // Get the block device that the user specifies is their floppy drive
    let cli = Cli::parse();

    // set the floppy disk path
    *FLOPPY_PATH.lock().expect("Fluster! Is single threaded.") = PathBuf::from(cli.block_device_path);

    // Set the virtual disk flag
    *USE_VIRTUAL_DISKS.lock().expect("Fluster! Is single threaded.") = cli.use_virtual_disks.unwrap_or(false);

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
    }).unwrap();






    // Check if the mount point is valid
    std::fs::create_dir_all(&mount_point).unwrap();

    let filesystem: FlusterFS = FlusterFS::new();

    // Mount it
    easy_fuser::mount(filesystem, &mount_point, &[]).unwrap();
}