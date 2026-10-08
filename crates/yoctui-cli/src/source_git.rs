//! Event-driven source status probes never delay the input loop or apply stale results.
use super::*;
use notify::{EventKind, RecommendedWatcher};
use std::sync::mpsc::{Receiver, TryRecvError};

mod watcher;
use watcher::WatchSetup;

const FALLBACK_REFRESH: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(crate) struct SourceGitPoller {
    source: Option<PathBuf>,
    watched_build_dir: Option<PathBuf>,
    pending: Option<tokio::task::JoinHandle<yoctui_model::SourceGitStatus>>,
    next: Option<Instant>,
    watcher: Option<RecommendedWatcher>,
    watch_events: Option<Receiver<notify::Result<notify::Event>>>,
    watch_setup: Option<WatchSetup>,
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
            self.watcher = None;
            self.watch_events = None;
            self.watch_setup = source
                .clone()
                .map(|source| WatchSetup::spawn(source, build_dir.clone()));
            compatibility_workspace_action(
                app,
                Action::SourceGitStatusUpdated(yoctui_model::SourceGitStatus::Scanning),
            );
            changed = true;
        }

        if self
            .watch_setup
            .as_ref()
            .is_some_and(WatchSetup::is_finished)
        {
            let setup = self
                .watch_setup
                .take()
                .expect("finished watcher setup exists");
            if let Some(installed) = setup.finish().await {
                self.watcher = Some(installed.watcher);
                self.watch_events = Some(installed.events);
                // Cover edits made between the initial status and watch setup.
                self.refresh_after_pending = self.pending.is_some();
                self.next = Some(Instant::now());
            }
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
            if !matches!(
                app.source_git_status,
                yoctui_model::SourceGitStatus::Ready(_)
            ) {
                compatibility_workspace_action(
                    app,
                    Action::SourceGitStatusUpdated(yoctui_model::SourceGitStatus::Scanning),
                );
            }
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
#[path = "tests/source_git.rs"]
mod tests;
