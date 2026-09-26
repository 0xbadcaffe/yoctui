use super::*;
use yoctui_model::{SavedBuild, SavedBuildOutcome};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "yoctui-archive-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn record(id: usize) -> SavedBuild {
    SavedBuild {
        id: id.to_string(),
        target: "same-image".into(),
        machine: Some("qemuarm64".into()),
        source: Some("/source".into()),
        build_dir: Some("/build".into()),
        outcome: SavedBuildOutcome::Succeeded,
        saved_unix_ms: id as u64,
        started_unix_ms: Some(10),
        finished_unix_ms: Some(20),
        logs: vec![yoctui_model::SavedBuildLog {
            unix_ms: 15,
            severity: yoctui_model::Severity::Info,
            message: format!("build {id}"),
            recipe: None,
            task: None,
            path: None,
            build: None,
        }],
        tasks: Vec::new(),
        limitations: Vec::new(),
    }
}
mod archive_corrupt_oversized_and_symlink_data_is_rejected_without_overwrite;
mod archive_restart_load_requires_no_daemon_or_environment;
mod archive_round_trip_retains_distinct_builds_and_private_bounds;
mod errors_resolved_cleanup_removes_only_selected_archive_identity;
