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

pub struct StartupMetadata {
    cancel: Option<oneshot::Sender<()>>,
    result: oneshot::Receiver<Result<Option<Workspace>>>,
    task: tokio::task::JoinHandle<()>,
    pending: bool,
}

impl StartupMetadata {
    pub fn spawn<F, Fut>(scan: F) -> Self
    where
        F: FnOnce(oneshot::Receiver<()>) -> Fut,
        Fut: std::future::Future<Output = Result<Option<Workspace>>> + Send + 'static,
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

    pub fn try_result(&mut self) -> Option<Result<Option<Workspace>>> {
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

impl Drop for StartupMetadata {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_inventory_oversized_workspace_reports_error_and_keeps_journal_usable() {
        use yoctui_model::{DaemonGlobalState, DaemonModelInstanceId, DaemonStateLimits};
        use yoctui_protocol::daemon::{
            DaemonSnapshotJournal, DaemonSnapshotLimits, MAX_FRAME_BYTES,
        };
        let state = DaemonGlobalState::new(
            DaemonModelInstanceId([1; 16]),
            1,
            "fixture-boot".into(),
            DaemonStateLimits::default(),
        )
        .unwrap();
        let mut journal = DaemonSnapshotJournal::new(
            crate::daemon_protocol_snapshot(&state),
            DaemonSnapshotLimits::default(),
        )
        .unwrap();
        let mut workspace = Workspace::default();
        workspace
            .variables
            .insert("oversized".into(), "x".repeat(MAX_FRAME_BYTES));
        assert!(!publish_workspace(&mut journal, workspace).unwrap());
        assert!(journal.snapshot().build_events.is_empty());
        assert!(
            journal
                .snapshot()
                .recent_logs
                .last()
                .unwrap()
                .message
                .contains("snapshot bounds")
        );
        assert!(publish_workspace(&mut journal, Workspace::default()).unwrap());
        assert_eq!(journal.snapshot().build_events.len(), 1);
    }

    #[test]
    fn daemon_startup_unready_child_is_terminated_and_reaped() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        drop(crate::DaemonStartupChild {
            child,
            ready: false,
        });
        // The child was reaped by the guard; waitpid must report ECHILD.
        let result = unsafe { libc::waitpid(pid as i32, std::ptr::null_mut(), libc::WNOHANG) };
        assert_eq!(result, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }

    #[tokio::test]
    async fn daemon_startup_slow_inventory_does_not_block_and_can_cancel() {
        let (cleaned_tx, cleaned_rx) = oneshot::channel();
        let mut scan = StartupMetadata::spawn(|cancel| async move {
            let _ = cancel.await;
            let _ = cleaned_tx.send(());
            Ok(None)
        });
        assert!(scan.pending());
        assert!(scan.try_result().is_none());
        tokio::time::timeout(std::time::Duration::from_secs(1), scan.shutdown())
            .await
            .unwrap();
        cleaned_rx.await.unwrap();
        assert!(scan.try_result().unwrap().unwrap().is_none());
        assert!(!scan.pending());
        assert!(scan.try_result().is_none());
    }

    #[tokio::test]
    async fn daemon_startup_inventory_is_delivered_once() {
        let mut scan = StartupMetadata::spawn(|_| async { Ok(Some(Workspace::default())) });
        (&mut scan.task).await.unwrap();
        assert!(scan.try_result().unwrap().unwrap().is_some());
        assert!(!scan.pending());
        assert!(scan.try_result().is_none());
    }

    #[tokio::test]
    async fn daemon_startup_failed_worker_does_not_remain_pending() {
        let mut scan = StartupMetadata::spawn(|_| async { panic!("fixture scan failure") });
        assert!((&mut scan.task).await.is_err());
        assert!(scan.try_result().unwrap().is_err());
        assert!(!scan.pending());
    }
}
