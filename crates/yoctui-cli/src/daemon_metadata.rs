//! Single owned startup scan; inventory must never gate daemon IPC readiness.
use anyhow::Result;
use tokio::sync::oneshot;
use yoctui_model::Workspace;

pub fn publish_workspace(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    workspace: Workspace,
) -> Result<bool> {
    use yoctui_protocol::daemon::{DaemonEvent, JobId};
    let (Some(event), _) =
        crate::daemon_build_event(yoctui_bitbake::BackendEvent::Workspace(workspace), JobId(0))
    else {
        unreachable!("workspace event conversion")
    };
    match journal.publish(DaemonEvent::Build(event)) {
        Ok(_) => {
            crate::publish_startup_metadata_log(
                journal,
                "Initial workspace and recipe inventory ready",
                false,
            )?;
            Ok(true)
        }
        Err(error) => {
            crate::publish_startup_metadata_log(
                journal,
                &format!(
                    "Initial metadata scan could not be published within daemon snapshot bounds: {error}"
                ),
                true,
            )?;
            Ok(false)
        }
    }
}

pub struct StartupMetadata<T = Workspace> {
    cancel: Option<oneshot::Sender<()>>,
    result: oneshot::Receiver<Result<Option<T>>>,
    task: tokio::task::JoinHandle<()>,
    pending: bool,
}

impl<T: Send + 'static> StartupMetadata<T> {
    pub fn spawn<F, Fut>(scan: F) -> Self
    where
        F: FnOnce(oneshot::Receiver<()>) -> Fut,
        Fut: std::future::Future<Output = Result<Option<T>>> + Send + 'static,
    {
        let (cancel, cancelled) = oneshot::channel();
        let (send, result) = oneshot::channel();
        let scan = scan(cancelled);
        let task = tokio::spawn(async move {
            let _ = send.send(scan.await);
        });
        Self {
            cancel: Some(cancel),
            result,
            task,
            pending: true,
        }
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn try_result(&mut self) -> Option<Result<Option<T>>> {
        if !self.pending {
            return None;
        }
        match self.result.try_recv() {
            Ok(result) => {
                self.pending = false;
                Some(result)
            }
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => {
                self.pending = false;
                Some(Err(anyhow::anyhow!(
                    "startup metadata worker exited without a result"
                )))
            }
        }
    }

    pub async fn shutdown(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        // The worker owns a bounded interrupt/reap path; await it before exit.
        let _ = (&mut self.task).await;
    }
}

impl<T> Drop for StartupMetadata<T> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
}

#[cfg(test)]
#[path = "tests/daemon_metadata/mod.rs"]
mod tests;
