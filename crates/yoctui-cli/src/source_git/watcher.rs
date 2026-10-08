//! Bounded, asynchronous source watches: never recurse through build output.
use super::*;
use notify::{RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::Component,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::io::AsyncReadExt;

const MAX_GIT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_WATCH_DIRECTORIES: usize = 4096;

pub(super) struct WatchInstallation {
    pub(super) watcher: RecommendedWatcher,
    pub(super) events: Receiver<notify::Result<notify::Event>>,
}

pub(super) struct WatchSetup {
    cancelled: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<Option<WatchInstallation>>,
}

impl Drop for WatchSetup {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.task.abort();
    }
}

impl WatchSetup {
    pub(super) fn spawn(source: PathBuf, build_dir: Option<PathBuf>) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let task = tokio::spawn(async move {
            let tracked = git_output(&source, &["ls-files", "--cached", "-z"]).await?;
            let mut directories = tracked_directories(&source, build_dir.as_deref(), &tracked);
            if let Some(metadata) = git_output(
                &source,
                &[
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-dir",
                    "--git-common-dir",
                ],
            )
            .await
            {
                // Index/HEAD/ref changes matter; Git objects never do. Includes
                // both metadata roots for linked worktrees, without recursion.
                for root in String::from_utf8_lossy(&metadata)
                    .lines()
                    .map(PathBuf::from)
                {
                    if root.is_absolute() {
                        for suffix in [
                            "",
                            "refs",
                            "refs/heads",
                            "refs/remotes",
                            "refs/remotes/origin",
                        ] {
                            directories.insert(root.join(suffix));
                        }
                    }
                }
            }
            tokio::task::spawn_blocking(move || install(directories, worker_cancelled))
                .await
                .ok()
                .flatten()
        });
        Self { cancelled, task }
    }

    pub(super) fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub(super) async fn finish(mut self) -> Option<WatchInstallation> {
        (&mut self.task).await.ok().flatten()
    }
}

async fn git_output(source: &Path, arguments: &[&str]) -> Option<Vec<u8>> {
    let mut command = tokio::process::Command::new("git");
    command
        .args(["--no-optional-locks", "-C"])
        .arg(source)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().ok()?;
    let mut group = child.id().map(yoctui_utils::ProcessGroupGuard::new);
    let stdout = child.stdout.take()?;
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_GIT_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .ok()?;
        if bytes.len() as u64 > MAX_GIT_BYTES || !child.wait().await.ok()?.success() {
            return None;
        }
        Some(bytes)
    })
    .await
    .ok()
    .flatten();
    if result.is_some()
        && let Some(group) = &mut group
    {
        group.disarm();
    }
    result
}

fn tracked_directories(
    source: &Path,
    build_dir: Option<&Path>,
    tracked: &[u8],
) -> BTreeSet<PathBuf> {
    let mut directories = BTreeSet::from([source.to_path_buf()]);
    for record in tracked
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        #[cfg(unix)]
        let relative = Path::new(std::ffi::OsStr::from_bytes(record));
        #[cfg(not(unix))]
        let Ok(record) = std::str::from_utf8(record) else {
            continue;
        };
        #[cfg(not(unix))]
        let relative = Path::new(record);
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            continue;
        }
        let absolute = source.join(relative);
        if build_dir.is_some_and(|build| absolute.starts_with(build)) {
            continue;
        }
        for directory in absolute
            .ancestors()
            .skip(1)
            .take_while(|directory| *directory != source)
        {
            if directories.len() >= MAX_WATCH_DIRECTORIES {
                return directories;
            }
            directories.insert(directory.to_path_buf());
        }
    }
    directories
}

fn install(
    directories: BTreeSet<PathBuf>,
    cancelled: Arc<AtomicBool>,
) -> Option<WatchInstallation> {
    // Bound queued notifications as well as watch registrations. Lost bursts
    // still have the 30-second status fallback, not an unbounded memory queue.
    let (send, events) = std::sync::mpsc::sync_channel(256);
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if !event
            .as_ref()
            .is_ok_and(|event| matches!(event.kind, EventKind::Access(_)))
        {
            let _ = send.try_send(event);
        }
    })
    .ok()?;
    let mut installed = false;
    for directory in directories {
        if cancelled.load(Ordering::Relaxed) {
            return None;
        }
        installed |= watcher
            .watch(&directory, RecursiveMode::NonRecursive)
            .is_ok();
    }
    installed.then_some(WatchInstallation { watcher, events })
}

#[cfg(test)]
#[path = "../tests/source_git_watcher.rs"]
mod tests;
