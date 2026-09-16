//! Private bounded history I/O and background daemon checkpoints.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use yoctui_model::{Action, App, SavedBuildAction};
use yoctui_protocol::{
    build_archive::{BuildArchive, MAX_ARCHIVE_BYTES},
    daemon::{DaemonSnapshot, JobKind, LifecycleState},
};

fn directory(root: &Path) -> Result<PathBuf> {
    let dir = yoctui_protocol::daemon_persist::persist_paths_for(root)?
        .directory
        .join("build-history");
    fs::create_dir_all(&dir)?;
    let meta = fs::symlink_metadata(&dir)?;
    ensure!(
        meta.is_dir() && !meta.file_type().is_symlink() && meta.uid() == unsafe { libc::geteuid() },
        "unsafe build history directory"
    );
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}
fn read(root: &Path) -> Result<BuildArchive> {
    let path = root.join("yoctui/build-history/history.json");
    if matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
        let paths = yoctui_protocol::daemon_persist::persist_paths_for(root)?;
        let builds = yoctui_protocol::daemon_persist::read_persisted_state(&paths)?
            .map(|state| yoctui_app::legacy_saved_builds(&state))
            .unwrap_or_default();
        return Ok(BuildArchive {
            schema_version: 1,
            builds,
        });
    }
    let dir = fs::symlink_metadata(root.join("yoctui/build-history"))?;
    ensure!(
        dir.is_dir() && !dir.file_type().is_symlink(),
        "unsafe build history directory"
    );
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o077 == 0,
        "unsafe build history file permissions"
    );
    let mut bytes = Vec::new();
    file.take((MAX_ARCHIVE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_ARCHIVE_BYTES,
        "build history exceeds 8 MiB"
    );
    let archive: BuildArchive = serde_json::from_slice(&bytes).context("corrupt build history")?;
    archive.validate().map_err(anyhow::Error::msg)?;
    Ok(archive)
}
fn save(root: &Path, record: yoctui_model::SavedBuild) -> Result<()> {
    let dir = directory(root)?;
    let mut archive = read(root)?;
    archive.remember(record);
    archive.validate().map_err(anyhow::Error::msg)?;
    let bytes = serde_json::to_vec(&archive)?;
    ensure!(
        bytes.len() <= MAX_ARCHIVE_BYTES,
        "build history exceeds 8 MiB"
    );
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = dir.join(format!(
        "history.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, dir.join("history.json"))?;
        fs::File::open(&dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub struct Recorder {
    root: PathBuf,
    pending: Option<tokio::task::JoinHandle<Result<()>>>,
    previous: Option<(u64, LifecycleState, Option<i32>)>,
    next: Instant,
}
impl Recorder {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            pending: None,
            previous: None,
            next: Instant::now(),
        }
    }
    pub async fn poll(&mut self, snapshot: &DaemonSnapshot, now: u64) {
        if self.pending.as_ref().is_some_and(|p| !p.is_finished()) {
            return;
        }
        if let Some(p) = self.pending.take()
            && let Err(e) = p.await.unwrap_or_else(|e| Err(e.into()))
        {
            self.previous = None;
            self.next = Instant::now() + Duration::from_secs(30);
            tracing::warn!(%e,"Saved build checkpoint failed");
        }
        if self.previous.is_none() && Instant::now() < self.next {
            return;
        }
        if !snapshot.build_events.iter().any(|event| {
            matches!(
                event,
                yoctui_protocol::daemon::DaemonBuildEvent::Reset { .. }
            )
        }) {
            return;
        }
        let Some(job) = snapshot
            .jobs
            .iter()
            .filter(|j| j.kind == JobKind::BitBakeBuild)
            .max_by_key(|j| j.id.0)
        else {
            return;
        };
        let key = (job.id.0, job.lifecycle, job.exit_code);
        let terminal = matches!(
            job.lifecycle,
            LifecycleState::Exited | LifecycleState::Failed | LifecycleState::Lost
        );
        if self.previous == Some(key) && (terminal || Instant::now() < self.next) {
            return;
        }
        let Some(record) = yoctui_app::capture_saved_build(snapshot, now) else {
            return;
        };
        self.previous = Some(key);
        self.next = Instant::now() + Duration::from_secs(30);
        let root = self.root.clone();
        self.pending = Some(tokio::task::spawn_blocking(move || save(&root, record)));
    }
    pub async fn finish(&mut self) {
        if let Some(p) = self.pending.take()
            && let Err(e) = p.await.unwrap_or_else(|e| Err(e.into()))
        {
            tracing::warn!(%e,"Saved build checkpoint failed");
        }
    }
}

pub type Load = tokio::task::JoinHandle<Result<BuildArchive>>;
pub async fn poll_load(app: &mut App, pending: &mut Option<Load>, root: &Path) -> bool {
    if pending.as_ref().is_some_and(|p| p.is_finished()) {
        let result = pending.take().unwrap().await;
        let (records, notice) = match result {
            Ok(Ok(archive)) => (archive.builds, None),
            Ok(Err(error)) => (
                Vec::new(),
                Some(format!("Saved history unavailable: {error}")),
            ),
            Err(error) => (
                Vec::new(),
                Some(format!("Saved history load failed: {error}")),
            ),
        };
        yoctui_model::update(
            app,
            Action::SavedBuild(SavedBuildAction::Loaded { records, notice }),
        );
        return true;
    }
    if app.saved_builds.reload_requested && pending.is_none() {
        app.saved_builds.reload_requested = false;
        app.saved_builds.loading = true;
        let root = root.to_owned();
        *pending = Some(tokio::task::spawn_blocking(move || read(&root)));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
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
            }],
            tasks: Vec::new(),
            limitations: Vec::new(),
        }
    }
    #[test]
    fn archive_round_trip_retains_distinct_builds_and_private_bounds() {
        let root = Root::new();
        assert!(read(&root.0).unwrap().builds.is_empty());
        for id in 0..35 {
            save(&root.0, record(id)).unwrap();
        }
        let archive = read(&root.0).unwrap();
        assert_eq!(archive.builds.len(), 32);
        assert_eq!(archive.builds[0].id, "34");
        assert_eq!(archive.builds[31].id, "3");
        assert_ne!(archive.builds[0].logs, archive.builds[1].logs);
        let file = root.0.join("yoctui/build-history/history.json");
        assert_eq!(fs::metadata(file).unwrap().mode() & 0o777, 0o600);
        save(&root.0, record(34)).unwrap();
        assert_eq!(read(&root.0).unwrap().builds.len(), 32);
    }
    #[test]
    fn archive_corrupt_oversized_and_symlink_data_is_rejected_without_overwrite() {
        let root = Root::new();
        save(&root.0, record(1)).unwrap();
        let file = root.0.join("yoctui/build-history/history.json");
        fs::write(&file, b"broken JSON").unwrap();
        assert!(save(&root.0, record(2)).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"broken JSON");
        fs::OpenOptions::new()
            .write(true)
            .open(&file)
            .unwrap()
            .set_len((MAX_ARCHIVE_BYTES + 1) as u64)
            .unwrap();
        assert!(read(&root.0).is_err());
        fs::remove_file(&file).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &file).unwrap();
        assert!(read(&root.0).is_err());
    }
    #[tokio::test]
    async fn archive_restart_load_requires_no_daemon_or_environment() {
        let root = Root::new();
        save(&root.0, record(1)).unwrap();
        let mut app = App::new_unconfigured(32, 4096);
        app.require_daemon = true;
        app.saved_builds.reload_requested = true;
        let mut load = None;
        assert!(poll_load(&mut app, &mut load, &root.0).await);
        tokio::time::timeout(Duration::from_secs(2), async {
            while load.is_some() {
                poll_load(&mut app, &mut load, &root.0).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(app.saved_builds.records[0].logs[0].message, "build 1");
        assert_eq!(
            app.daemon.status,
            yoctui_model::ClientReplicaStatus::Disconnected
        );
        assert!(matches!(
            app.build_environment,
            yoctui_model::BuildEnvironmentState::Unconfigured
        ));
        assert!(app.tasks.is_empty());
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    use yoctui_protocol::daemon::*;
    #[tokio::test]
    async fn archive_daemon_checkpoint_survives_without_attached_clients() {
        let root =
            std::env::temp_dir().join(format!("yoctui-archive-checkpoint-{}", std::process::id()));
        let mut snapshot = DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([2; 16]),
            sequence: 1,
            generation: 1,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: vec![JobSummary {
                id: JobId(4),
                kind: JobKind::BitBakeBuild,
                label: "image".into(),
                lifecycle: LifecycleState::Running,
                progress_current: None,
                progress_total: None,
                exit_code: None,
            }],
            raw_executions: Vec::new(),
            raw_history: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_progress: None,
            recovery_warnings: Vec::new(),
            build_events: vec![
                DaemonBuildEvent::Reset {
                    targets: vec!["image".into()],
                },
                DaemonBuildEvent::Started {
                    started_unix_ms: Some(100),
                },
            ],
        };
        let mut recorder = Recorder::new(root.clone());
        recorder.poll(&snapshot, 200).await;
        recorder.finish().await;
        assert_eq!(
            read(&root).unwrap().builds[0].outcome,
            yoctui_model::SavedBuildOutcome::Incomplete
        );
        snapshot.jobs[0].lifecycle = LifecycleState::Exited;
        snapshot.jobs[0].exit_code = Some(0);
        snapshot.build_events.push(DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: Some(300),
        });
        recorder.poll(&snapshot, 300).await;
        recorder.finish().await;
        let archive = read(&root).unwrap();
        assert_eq!(archive.builds.len(), 1);
        assert_eq!(
            archive.builds[0].outcome,
            yoctui_model::SavedBuildOutcome::Succeeded
        );
        assert!(
            archive.builds[0]
                .limitations
                .iter()
                .any(|s| s.contains("unavailable"))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
