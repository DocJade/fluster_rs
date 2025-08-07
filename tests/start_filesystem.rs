use test_log::test; // We want to see logs while testing.
pub mod test_common;

#[test]
// Try starting up the filesystem
fn filesystem_starts() {
    test_common::start_filesystem();
}
