use std::path::PathBuf;

use thiserror::Error;
use tokio::sync::mpsc;
use yoctui_bitbake::{SdkToolAdapter, SdkToolJobRunner, SdkToolRunnerEvent};
use yoctui_model::{
    SdkArtifactIdentity, SdkNativeMode, SdkNativePreview, SdkNativeRequest, SdkOutputStream,
    SdkPublishPreview,
};
use yoctui_protocol::daemon::{DaemonSdkContext, DaemonSdkNativeMode, DaemonSdkOperation, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonSdkEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Cancelled {
        job_id: JobId,
        session_id: u64,
        forced: bool,
        exit_code: Option<i32>,
    },
    TimedOut {
        job_id: JobId,
        session_id: u64,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        session_id: u64,
        message: String,
    },
}

pub struct DaemonSdkSupervisor {
    next_job_id: u64,
    active: std::collections::HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonSdkEvent>,
    rx: mpsc::UnboundedReceiver<DaemonSdkEvent>,
}

impl Default for DaemonSdkSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: std::collections::HashMap::new(),
            tx,
            rx,
        }
    }
}

impl DaemonSdkSupervisor {
    pub fn start(
        &mut self,
        session_id: u64,
        operation: DaemonSdkOperation,
        context: DaemonSdkContext,
    ) -> Result<JobId, DaemonSdkError> {
        let (command, cwd) = sdk_command(operation.clone(), context.clone())?;
        let sdk_deploy_root = context.sdk_deploy_root.clone();
        let workspace_roots = context.workspace_roots.clone();
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(DaemonSdkError::JobSpaceExhausted)?;
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = SdkToolJobRunner::new();
            let adapter = SdkToolAdapter::new(
                cwd.clone(),
                sdk_deploy_root.into(),
                workspace_roots.into_iter().map(PathBuf::from).collect(),
            );
            let preview = match command {
                SdkCommand::Publish(preview) => adapter.publication_command(&preview),
                SdkCommand::Native(preview) => adapter.native_command(&preview),
            };
            let command = match preview {
                Ok(command) => command,
                Err(error) => {
                    let _ = tx.send(DaemonSdkEvent::Lost {
                        job_id,
                        session_id,
                        message: error.to_string(),
                    });
                    return;
                }
            };
            if let Err(error) = runner.start(command).await {
                let _ = tx.send(DaemonSdkEvent::Lost {
                    job_id,
                    session_id,
                    message: error.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {
                    cancellation = cancel_rx.recv() => {
                        if cancellation.is_some() { let _ = runner.cancel().await; }
                    }
                    event = runner.next_event() => {
                        let terminal = match event {
                            Ok(SdkToolRunnerEvent::Started) => tx.send(DaemonSdkEvent::Started { job_id, session_id }),
                            Ok(SdkToolRunnerEvent::Output { stream, line, truncated }) => tx.send(DaemonSdkEvent::Output { job_id, session_id, stream, line, truncated }),
                            Ok(SdkToolRunnerEvent::Completed { exit_code }) => tx.send(DaemonSdkEvent::Completed { job_id, session_id, exit_code }),
                            Ok(SdkToolRunnerEvent::Failed { exit_code }) => tx.send(DaemonSdkEvent::Failed { job_id, session_id, exit_code }),
                            Ok(SdkToolRunnerEvent::Cancelled { forced, exit_code }) => tx.send(DaemonSdkEvent::Cancelled { job_id, session_id, forced, exit_code }),
                            Ok(SdkToolRunnerEvent::TimedOut { forced, exit_code }) => tx.send(DaemonSdkEvent::TimedOut { job_id, session_id, forced, exit_code }),
                            Ok(SdkToolRunnerEvent::CancellationRejected { message }) => tx.send(DaemonSdkEvent::Lost { job_id, session_id, message }),
                            Ok(SdkToolRunnerEvent::Lost { message }) => tx.send(DaemonSdkEvent::Lost { job_id, session_id, message }),
                            Err(error) => tx.send(DaemonSdkEvent::Lost { job_id, session_id, message: error.to_string() }),
                        };
                        if terminal.is_err() || !runner.is_active() { return; }
                    }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, session_id: u64) -> Result<(), DaemonSdkError> {
        self.active
            .get(&session_id)
            .ok_or(DaemonSdkError::UnknownSession(session_id))?
            .send(())
            .map_err(|_| DaemonSdkError::UnknownSession(session_id))
    }

    pub fn try_event(&mut self) -> Option<DaemonSdkEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonSdkEvent::Completed { .. }
                | DaemonSdkEvent::Failed { .. }
                | DaemonSdkEvent::Cancelled { .. }
                | DaemonSdkEvent::TimedOut { .. }
                | DaemonSdkEvent::Lost { .. }
        ) {
            self.active.remove(&event.session_id());
        }
        Some(event)
    }
}

impl DaemonSdkEvent {
    pub fn job_id(&self) -> JobId {
        match self {
            Self::Started { job_id, .. }
            | Self::Output { job_id, .. }
            | Self::Completed { job_id, .. }
            | Self::Failed { job_id, .. }
            | Self::Cancelled { job_id, .. }
            | Self::TimedOut { job_id, .. }
            | Self::Lost { job_id, .. } => *job_id,
        }
    }

    fn session_id(&self) -> u64 {
        match self {
            Self::Started { session_id, .. }
            | Self::Output { session_id, .. }
            | Self::Completed { session_id, .. }
            | Self::Failed { session_id, .. }
            | Self::Cancelled { session_id, .. }
            | Self::TimedOut { session_id, .. }
            | Self::Lost { session_id, .. } => *session_id,
        }
    }
}

enum SdkCommand {
    Publish(SdkPublishPreview),
    Native(SdkNativePreview),
}

fn sdk_command(
    operation: DaemonSdkOperation,
    context: DaemonSdkContext,
) -> Result<(SdkCommand, PathBuf), DaemonSdkError> {
    let build = PathBuf::from(context.build_directory);
    match operation {
        DaemonSdkOperation::Publish {
            executable,
            artifact,
            destination,
        } => {
            let artifact = SdkArtifactIdentity {
                path: artifact.path.into(),
                size_bytes: artifact.size_bytes,
                modified_unix_seconds: artifact.modified_unix_seconds,
            };
            let preview = SdkPublishPreview::new(executable.into(), artifact, destination.into())
                .map_err(DaemonSdkError::InvalidPreview)?;
            Ok((SdkCommand::Publish(preview), build))
        }
        DaemonSdkOperation::Native {
            executable,
            mode,
            extracted_root,
            recipe,
            tool,
            arguments,
        } => {
            let preview = SdkNativePreview::new(SdkNativeRequest {
                executable: executable.into(),
                mode: match mode {
                    DaemonSdkNativeMode::FindSysroot => SdkNativeMode::FindSysroot,
                    DaemonSdkNativeMode::RunNative => SdkNativeMode::RunNative,
                },
                extracted_root: extracted_root.map(Into::into),
                recipe,
                tool,
                arguments,
            })
            .map_err(DaemonSdkError::InvalidPreview)?;
            Ok((SdkCommand::Native(preview), build))
        }
    }
}

#[derive(Debug, Error)]
pub enum DaemonSdkError {
    #[error("SDK job ID space exhausted")]
    JobSpaceExhausted,
    #[error("unknown SDK session {0}")]
    UnknownSession(u64),
    #[error("invalid SDK preview: {0}")]
    InvalidPreview(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_runtime_sdk_wire_operations_preserve_closed_identity() {
        let op = DaemonSdkOperation::Native {
            executable: "/sdk/oe-run-native".into(),
            mode: DaemonSdkNativeMode::RunNative,
            extracted_root: None,
            recipe: "cmake-native".into(),
            tool: Some("cmake".into()),
            arguments: vec!["--version".into()],
        };
        let context = DaemonSdkContext {
            build_directory: "/build".into(),
            sdk_deploy_root: "/deploy/sdk".into(),
            workspace_roots: vec!["/sdk".into()],
        };
        let (SdkCommand::Native(preview), cwd) = sdk_command(op, context).unwrap() else {
            panic!()
        };
        assert_eq!(cwd, PathBuf::from("/build"));
        assert_eq!(preview.argv[1], PathBuf::from("cmake-native"));
    }
}
