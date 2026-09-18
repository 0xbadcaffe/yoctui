use super::*;

#[cfg(unix)]
pub(crate) fn write_test_executable(path: &Path, body: &str) {
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_EXECUTABLE: AtomicU64 = AtomicU64::new(1);

    let temporary = path.with_extension(format!(
        "yoctui-fixture-write-{}-{}",
        std::process::id(),
        NEXT_EXECUTABLE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temporary, body).unwrap();
    let mut permissions = fs::metadata(&temporary).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&temporary, permissions).unwrap();
    fs::rename(temporary, path).unwrap();
}
