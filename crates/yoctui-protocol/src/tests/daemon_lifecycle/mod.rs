use super::*;
use crate::daemon_ipc::{DaemonListener, RUNTIME_DIRECTORY_MODE, runtime_paths_for};
use std::{env, os::unix::fs::DirBuilderExt, time::SystemTime};

fn paths(name: &str) -> RuntimePaths {
    let root = env::temp_dir().join(format!(
        "yoctui-daemon-lifecycle-{name}-{}",
        std::process::id(),
    ));
    let _ = fs::remove_dir_all(&root);
    fs::DirBuilder::new()
        .mode(RUNTIME_DIRECTORY_MODE)
        .create(&root)
        .unwrap();
    // SAFETY: geteuid has no preconditions.
    runtime_paths_for(root, unsafe { libc::geteuid() }).unwrap()
}

mod daemon_lifecycle_runtime_record_is_private_atomic_and_instance_guarded;

mod daemon_lifecycle_accepts_only_exact_replaced_executable_identity;

mod daemon_lifecycle_rejects_symlink_and_oversized_runtime_records;
