//! Event-driven source status probes never delay the input loop or apply stale results.
use super::*;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{Receiver, TryRecvError};

const FALLBACK_REFRESH: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(crate) struct SourceGitPoller {
    source: Option<PathBuf>,
    watched_build_dir: Option<PathBuf>,
    pending: Option<tokio::task::JoinHandle<yoctui_model::SourceGitStatus>>,
    next: Option<Instant>,
    watcher: Option<RecommendedWatcher>,
    watch_events: Option<Receiver<notify::Result<notify::Event>>>,
    refresh_after_pending: bool,
}

impl Drop for SourceGitPoller {
    fn drop(&mut self) {
        if let Some(task) = self.pending.take() {
            task.abort();
        }
    }
}

impl SourceGitPoller {
    pub(crate) async fn poll(&mut self, app: &mut App) -> bool {
        let source = app.source_repository_path().map(Path::to_owned);
        let build_dir = app.workspace.build_dir.clone();
        let mut changed = false;
        if source != self.source || build_dir != self.watched_build_dir {
            if let Some(task) = self.pending.take() {
                task.abort();
            }
            self.source = source.clone();
            self.watched_build_dir = build_dir.clone();
            self.next = None;
            self.refresh_after_pending = false;
            self.install_watcher(source.as_deref(), build_dir.as_deref());
            compatibility_workspace_action(
                app,
                Action::SourceGitStatusUpdated(yoctui_model::SourceGitStatus::Scanning),
            );
            changed = true;
        }

        if self.drain_relevant_events(app.workspace.build_dir.as_deref()) {
            if self.pending.is_some() {
                self.refresh_after_pending = true;
            } else {
                self.next = Some(Instant::now());
            }
        }

        if self.pending.as_ref().is_some_and(|task| task.is_finished()) {
            let result = self
                .pending
                .take()
                .expect("finished source Git task exists")
                .await
                .unwrap_or_else(|error| {
                    yoctui_model::SourceGitStatus::Unavailable(error.to_string())
                });
            compatibility_workspace_action(app, Action::SourceGitStatusUpdated(result));
            self.next = Some(if std::mem::take(&mut self.refresh_after_pending) {
                Instant::now()
            } else {
                Instant::now() + FALLBACK_REFRESH
            });
            changed = true;
            return changed;
        }

        if self.pending.is_none() && self.next.is_none_or(|next| Instant::now() >= next) {
            compatibility_workspace_action(
                app,
                Action::SourceGitStatusUpdated(yoctui_model::SourceGitStatus::Scanning),
            );
            self.pending = Some(tokio::spawn(async move {
                match source {
                    Some(source) => yoctui_bitbake::inspect_source_git(&source).await,
                    None => yoctui_model::SourceGitStatus::Unavailable(
                        "Select a source directory in Build Environment".into(),
                    ),
                }
            }));
            changed = true;
        }
        changed
    }

    fn install_watcher(&mut self, source: Option<&Path>, build_dir: Option<&Path>) {
        self.watcher = None;
        self.watch_events = None;
        let Some(source) = source else {
            return;
        };
        let (send, receive) = std::sync::mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(move |event| {
            let _ = send.send(event);
        }) else {
            return;
        };
        let mut installed = watcher.watch(source, RecursiveMode::NonRecursive).is_ok();
        if let Ok(entries) = source.read_dir() {
            for path in entries.flatten().map(|entry| entry.path()) {
                if !path.is_dir() || build_dir.is_some_and(|build| build.starts_with(&path)) {
                    continue;
                }
                installed |= watcher.watch(&path, RecursiveMode::Recursive).is_ok();
            }
        }
        if installed {
            self.watcher = Some(watcher);
            self.watch_events = Some(receive);
        }
    }

    fn drain_relevant_events(&mut self, build_dir: Option<&Path>) -> bool {
        let Some(events) = &self.watch_events else {
            return false;
        };
        let source = self.source.as_deref();
        let mut relevant = false;
        loop {
            match events.try_recv() {
                Ok(Ok(event)) => {
                    if !matches!(event.kind, EventKind::Access(_)) {
                        relevant |= event.paths.is_empty()
                            || event.paths.iter().any(|path| {
                                !build_dir.is_some_and(|build| path.starts_with(build))
                                    && !source.is_some_and(|root| {
                                        path.starts_with(root.join(".git/objects"))
                                    })
                            });
                    }
                }
                Ok(Err(_)) => relevant = true,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        relevant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, arguments: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn source_git_refreshes_after_a_worktree_event() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-source-watch-{}-{}",
            std::process::id(),
            yoctui_utils::unix_ms()
        ));
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.name", "Yoctui Test"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        std::fs::write(root.join("tracked.txt"), "clean\n").unwrap();
        git(&root, &["add", "tracked.txt"]);
        git(&root, &["commit", "-qm", "initial"]);

        let mut app = App::new(10, 1_000);
        app.workspace.source_dir = Some(root.clone());
        let mut poller = SourceGitPoller::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(_)
        ) {
            poller.poll(&mut app).await;
            assert!(Instant::now() < deadline, "initial Git status timed out");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let quiet_deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < quiet_deadline {
            assert!(
                !poller.poll(&mut app).await,
                "a read-only Git status probe must not trigger itself"
            );
            assert!(matches!(
                app.source_git_status,
                yoctui_model::SourceGitStatus::Ready(_)
            ));
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        std::fs::write(root.join("tracked.txt"), "dirty\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            poller.poll(&mut app).await;
            if matches!(
                &app.source_git_status,
                yoctui_model::SourceGitStatus::Ready(summary) if summary.unstaged == 1
            ) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "worktree event did not refresh Git status"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
