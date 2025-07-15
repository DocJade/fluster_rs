fn main() {
    #[cfg(target_os = "windows")]
    panic!("The `fuser` crate cannot be built on windows. You must build and use fluster_fs through WSL.");
}