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
#[path = "tests/build_archive/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/build_archive_checkpoint/mod.rs"]
mod checkpoint_tests;
