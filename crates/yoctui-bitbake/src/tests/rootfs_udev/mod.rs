use super::*;

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("yoctui-udev-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

mod udev_inventory_preserves_overrides_masks_and_bounded_preview;

#[cfg(unix)]
mod udev_image_absolute_links_stay_in_image_and_loops_are_bounded;

mod udev_empty_scan_cancellation_and_deadline_are_explicit;

#[cfg(unix)]
mod udev_unresolved_file_is_retained_and_limits_are_reported;
