//! Periodic source probes never delay the event loop or apply stale results.
use super::*;
#[derive(Default)]
pub(crate) struct SourceGitPoller {
    source: Option<PathBuf>,
    pending: Option<tokio::task::JoinHandle<yoctui_model::SourceGitStatus>>,
    next: Option<Instant>,
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
        let mut changed = false;
        if source != self.source {
            if let Some(task) = self.pending.take() {
                task.abort();
            }
            self.source = source.clone();
            self.next = None;
            compatibility_workspace_action(
                app,
                Action::SourceGitStatusUpdated(yoctui_model::SourceGitStatus::Scanning),
            );
            changed = true;
        }
        if self.pending.as_ref().is_some_and(|task| task.is_finished()) {
            let result = self
                .pending
                .take()
                .unwrap()
                .await
                .unwrap_or_else(|e| yoctui_model::SourceGitStatus::Unavailable(e.to_string()));
            compatibility_workspace_action(app, Action::SourceGitStatusUpdated(result));
            self.next = Some(Instant::now() + Duration::from_secs(5));
            changed = true;
        }
        if self.pending.is_none() && self.next.is_none_or(|next| Instant::now() >= next) {
            self.pending = Some(tokio::spawn(async move {
                match source {
                    Some(source) => yoctui_bitbake::inspect_source_git(&source).await,
                    None => yoctui_model::SourceGitStatus::Unavailable(
                        "Select a source directory in Build Environment".into(),
                    ),
                }
            }));
        }
        changed
    }
}
