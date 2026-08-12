use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use thiserror::Error;
use tokio::sync::mpsc;
use yoctui_bitbake::{
    DevtoolCommandSpec, DevtoolJobRunner, DevtoolOutputStream, DevtoolRunnerEvent,
};
use yoctui_model::DevtoolOperation;
use yoctui_protocol::daemon::{DaemonDevtoolOperation, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonDevtoolEvent {
    Started {
        job_id: JobId,
        label: String,
    },
    Output {
        job_id: JobId,
        stream: DevtoolOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        exit_code: Option<i32>,
    },
    Cancelled {
        job_id: JobId,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        message: String,
    },
}

pub struct DaemonDevtoolSupervisor {
    executable: PathBuf,
    cancellation_timeout: Duration,
    next_job_id: u64,
    active: HashMap<JobId, mpsc::UnboundedSender<()>>,
    events_tx: mpsc::UnboundedSender<DaemonDevtoolEvent>,
    events_rx: mpsc::UnboundedReceiver<DaemonDevtoolEvent>,
}

impl Default for DaemonDevtoolSupervisor {
    fn default() -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        Self {
            executable: PathBuf::from("devtool"),
            cancellation_timeout: Duration::from_secs(5),
            next_job_id: 1,
            active: HashMap::new(),
            events_tx,
            events_rx,
        }
    }
}

impl DaemonDevtoolSupervisor {
    #[cfg(test)]
    pub fn with_executable(mut self, executable: PathBuf) -> Self {
        self.executable = executable;
        self
    }

    pub fn start(
        &mut self,
        operation: DaemonDevtoolOperation,
        build_directory: PathBuf,
    ) -> Result<JobId, DaemonDevtoolError> {
        let build_directory = canonical_build_directory(build_directory)?;
        let operation = model_operation(operation)?;
        let command = DevtoolCommandSpec::with_executable(self.executable.clone(), &operation)?;
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(DaemonDevtoolError::JobSpaceExhausted)?;
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(job_id, cancel_tx);
        let events = self.events_tx.clone();
        let cancellation_timeout = self.cancellation_timeout;
        tokio::spawn(async move {
            let label = format!("Devtool {}", operation.recipe());
            let mut runner = DevtoolJobRunner::new(build_directory)
                .with_cancellation_timeout(cancellation_timeout);
            if let Err(error) = runner.start(command).await {
                let _ = events.send(DaemonDevtoolEvent::Lost {
                    job_id,
                    message: error.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {
                    cancellation = cancel_rx.recv() => {
                        if cancellation.is_some()
                            && let Err(error) = runner.cancel().await
                        {
                            let _ = events.send(DaemonDevtoolEvent::Lost {
                                job_id,
                                message: error.to_string(),
                            });
                            return;
                        }
                    }
                    event = runner.next_event() => {
                        let terminal = match event {
                            Ok(DevtoolRunnerEvent::Started) => events.send(DaemonDevtoolEvent::Started { job_id, label: label.clone() }),
                            Ok(DevtoolRunnerEvent::Output { stream, line, truncated }) => events.send(DaemonDevtoolEvent::Output { job_id, stream, line, truncated }),
                            Ok(DevtoolRunnerEvent::Completed { exit_code }) => events.send(DaemonDevtoolEvent::Completed { job_id, exit_code }),
                            Ok(DevtoolRunnerEvent::Failed { exit_code }) => events.send(DaemonDevtoolEvent::Failed { job_id, exit_code }),
                            Ok(DevtoolRunnerEvent::Cancelled { forced, exit_code }) => events.send(DaemonDevtoolEvent::Cancelled { job_id, forced, exit_code }),
                            Ok(DevtoolRunnerEvent::Lost { message }) => events.send(DaemonDevtoolEvent::Lost { job_id, message }),
                            Err(error) => events.send(DaemonDevtoolEvent::Lost { job_id, message: error.to_string() }),
                        };
                        let is_terminal = matches!(
                            terminal,
                            Ok(())
                        ) && !runner.is_active();
                        if terminal.is_err() || is_terminal {
                            return;
                        }
                    }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, job_id: JobId) -> Result<(), DaemonDevtoolError> {
        self.active
            .get(&job_id)
            .ok_or(DaemonDevtoolError::UnknownJob(job_id))?
            .send(())
            .map_err(|_| DaemonDevtoolError::UnknownJob(job_id))
    }

    pub fn try_event(&mut self) -> Option<DaemonDevtoolEvent> {
        let event = self.events_rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonDevtoolEvent::Completed { .. }
                | DaemonDevtoolEvent::Failed { .. }
                | DaemonDevtoolEvent::Cancelled { .. }
                | DaemonDevtoolEvent::Lost { .. }
        ) {
            self.active.remove(&event.job_id());
        }
        Some(event)
    }
}

impl DaemonDevtoolEvent {
    pub fn job_id(&self) -> JobId {
        match self {
            Self::Started { job_id, .. }
            | Self::Output { job_id, .. }
            | Self::Completed { job_id, .. }
            | Self::Failed { job_id, .. }
            | Self::Cancelled { job_id, .. }
            | Self::Lost { job_id, .. } => *job_id,
        }
    }
}

fn canonical_build_directory(path: PathBuf) -> Result<PathBuf, DaemonDevtoolError> {
    if !path.is_absolute() {
        return Err(DaemonDevtoolError::UnsafeBuildDirectory(path));
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| DaemonDevtoolError::UnsafeBuildDirectory(path.clone()))?;
    let canonical = fs::canonicalize(&path)
        .map_err(|_| DaemonDevtoolError::UnsafeBuildDirectory(path.clone()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || canonical != path {
        return Err(DaemonDevtoolError::UnsafeBuildDirectory(path));
    }
    Ok(canonical)
}

fn model_operation(
    operation: DaemonDevtoolOperation,
) -> Result<DevtoolOperation, DaemonDevtoolError> {
    let operation = match operation {
        DaemonDevtoolOperation::Modify { recipe } => DevtoolOperation::Modify { recipe },
        DaemonDevtoolOperation::UpdateRecipe { recipe } => {
            DevtoolOperation::UpdateRecipe { recipe }
        }
        DaemonDevtoolOperation::Finish {
            recipe,
            destination,
        } => DevtoolOperation::Finish {
            recipe,
            destination: destination.into(),
        },
        DaemonDevtoolOperation::DeployTarget { recipe, target } => {
            DevtoolOperation::DeployTarget { recipe, target }
        }
        DaemonDevtoolOperation::UndeployTarget { recipe, target } => {
            DevtoolOperation::UndeployTarget { recipe, target }
        }
        DaemonDevtoolOperation::Reset { recipe } => DevtoolOperation::Reset { recipe },
    };
    operation.validate()?;
    Ok(operation)
}

#[derive(Debug, Error)]
pub enum DaemonDevtoolError {
    #[error("unsafe Devtool build directory: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("Devtool job ID space exhausted")]
    JobSpaceExhausted,
    #[error("unknown daemon Devtool job {0:?}")]
    UnknownJob(JobId),
    #[error(transparent)]
    Operation(#[from] yoctui_model::DevtoolOperationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn client_runtime_devtool_runner_survives_client_scope_and_reports_terminal_event() {
        let root =
            std::env::temp_dir().join(format!("yoctui-daemon-devtool-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("devtool");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf 'started:%s\\n' \"$*\"\nsleep 0.05\nprintf 'finished\\n'\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let mut supervisor = DaemonDevtoolSupervisor::default().with_executable(executable);
        let job = supervisor
            .start(
                DaemonDevtoolOperation::Modify {
                    recipe: "busybox".into(),
                },
                root.clone(),
            )
            .unwrap();
        let client_connection = String::from("detached-client");
        drop(client_connection);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut saw_output = false;
        let terminal = loop {
            if let Some(event) = supervisor.try_event() {
                saw_output |= matches!(event, DaemonDevtoolEvent::Output { .. });
                if matches!(event, DaemonDevtoolEvent::Completed { .. }) {
                    break event;
                }
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(5)).await;
        };
        assert!(saw_output);
        assert_eq!(terminal.job_id(), job);
        fs::remove_dir_all(root).unwrap();
    }
}
