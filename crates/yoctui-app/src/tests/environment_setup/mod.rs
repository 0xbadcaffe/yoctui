use super::*;
mod tempfile {
    use super::*;
    pub struct TempDir(PathBuf);
    impl TempDir {
        pub fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    pub fn tempdir() -> std::io::Result<TempDir> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yoctui-environment-{}-{now}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(TempDir(path))
    }
}
mod environment_setup_directory_scan_is_bounded_and_empty_is_valid;
mod environment_setup_directory_scan_missing_build_hidden_and_split_source;
mod environment_setup_directory_symlinks_resolve_without_execution;
mod environment_setup_keys_trap_editing_and_route_pages;
