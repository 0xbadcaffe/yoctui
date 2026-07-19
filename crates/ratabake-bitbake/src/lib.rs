//! BitBake adapters. They execute BitBake; they never evaluate metadata themselves.
use async_trait::async_trait;
use ratabake_model::{BuildRequest, LogEntry, Severity, Workspace};
use ratabake_protocol::{
    Command, Envelope, Event, ProtocolError, VERSION, decode_line, encode_line,
};
use std::{path::PathBuf, process::Stdio, time::SystemTime};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command as TokioCommand},
};

async fn read_output<R>(stream: R, sender: tokio::sync::mpsc::Sender<LogEntry>)
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if sender.send(classify_output(line)).await.is_err() {
            break;
        }
    }
}
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("process: {0}")]
    Process(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("bridge: {0}")]
    Bridge(String),
    #[error("backend is not running")]
    NotRunning,
}
#[derive(Debug, Clone)]
pub enum BackendEvent {
    Workspace(Workspace),
    BuildStarted,
    ParseProgress,
    Log(LogEntry),
    TaskStarted {
        recipe: String,
        task: String,
    },
    TaskProgress {
        recipe: String,
        task: String,
        progress: u8,
    },
    TaskCompleted {
        recipe: String,
        task: String,
        success: bool,
    },
    BuildCompleted {
        success: bool,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    Disconnected,
}
#[async_trait]
pub trait BitBakeBackend: Send {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError>;
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError>;
    async fn cancel_build(&mut self) -> Result<(), BackendError>;
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError>;
}
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for x in chars.by_ref() {
                if x.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c)
        }
    }
    out
}
pub fn classify_output(line: String) -> LogEntry {
    let clean = strip_ansi(&line);
    let lower = clean.to_ascii_lowercase();
    let severity = if lower.contains("error:") || lower.starts_with("error") {
        Severity::Error
    } else if lower.contains("warning:") || lower.starts_with("warning") {
        Severity::Warning
    } else {
        Severity::Info
    };
    LogEntry {
        severity,
        message: clean,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
    }
}
pub struct ProcessBackend {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<LogEntry>>,
}
impl ProcessBackend {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
        }
    }
    async fn collect(&mut self) -> Result<bool, BackendError> {
        let child = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        let status = child.wait().await?;
        Ok(status.success())
    }
}
#[async_trait]
impl BitBakeBackend for ProcessBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        Ok(Workspace {
            build_dir: Some(self.build_dir.clone()),
            ..Workspace::default()
        })
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        request
            .validate()
            .map_err(|e| BackendError::Bridge(e.to_string()))?;
        let mut cmd = TokioCommand::new("bitbake");
        cmd.args(&request.targets)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or(BackendError::Bridge("stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(BackendError::Bridge("stderr unavailable".into()))?;
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        tokio::spawn(read_output(stdout, tx.clone()));
        tokio::spawn(read_output(stderr, tx.clone()));
        drop(tx);
        self.child = Some(child);
        self.output = Some(rx);
        Ok(())
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        let c = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        c.kill().await?;
        Ok(())
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        if let Some(output) = self.output.as_mut()
            && let Some(line) = output.recv().await
        {
            return Ok(BackendEvent::Log(line));
        }
        let success = self.collect().await?;
        Ok(BackendEvent::BuildCompleted { success })
    }
}
pub struct BridgeBackend {
    child: Child,
    stdin: ChildStdin,
    lines: BufReader<tokio::process::ChildStdout>,
    sequence: u64,
    last_sequence: u64,
}
impl BridgeBackend {
    pub async fn spawn(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
    ) -> Result<Self, BackendError> {
        let mut child = TokioCommand::new(python)
            .arg(script)
            .current_dir(build_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdout unavailable".into()))?;
        Ok(Self {
            child,
            stdin,
            lines: BufReader::new(stdout),
            sequence: 0,
            last_sequence: 0,
        })
    }
    async fn command(&mut self, message: Command) -> Result<(), BackendError> {
        self.sequence += 1;
        let bytes = encode_line(&Envelope {
            protocol_version: VERSION,
            sequence: self.sequence,
            correlation_id: Some(self.sequence.to_string()),
            message,
        })?;
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }
    fn event(event: Event) -> BackendEvent {
        match event {
            Event::Workspace { .. } => BackendEvent::Workspace(Workspace::default()),
            Event::BuildStarted => BackendEvent::BuildStarted,
            Event::ParseProgress { .. } => BackendEvent::ParseProgress,
            Event::TaskStarted { recipe, task, .. } => BackendEvent::TaskStarted { recipe, task },
            Event::TaskProgress {
                recipe,
                task,
                progress,
            } => BackendEvent::TaskProgress {
                recipe,
                task,
                progress: progress.unwrap_or(0),
            },
            Event::TaskCompleted {
                recipe,
                task,
                success,
            } => BackendEvent::TaskCompleted {
                recipe,
                task,
                success,
            },
            Event::Log {
                level,
                message,
                recipe,
                task,
                path,
            } => {
                let severity = match level.as_str() {
                    "warning" => Severity::Warning,
                    "error" => Severity::Error,
                    _ => Severity::Info,
                };
                BackendEvent::Log(LogEntry {
                    severity,
                    message,
                    recipe,
                    task,
                    path: path.map(PathBuf::from),
                    timestamp: SystemTime::now(),
                })
            }
            Event::Warning { message } => BackendEvent::Log(LogEntry {
                severity: Severity::Warning,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
            }),
            Event::Error { message } => BackendEvent::Log(LogEntry {
                severity: Severity::Error,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
            }),
            Event::BuildCompleted { success } => BackendEvent::BuildCompleted { success },
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                BackendEvent::CommandFailed { code, message }
            }
            Event::BridgeShutdown | Event::HelloAck { .. } | Event::Unknown => {
                BackendEvent::Disconnected
            }
        }
    }
}
#[async_trait]
impl BitBakeBackend for BridgeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        self.command(Command::InspectWorkspace).await?;
        Ok(Workspace::default())
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let mut line = Vec::new();
        if self.lines.read_until(b'\n', &mut line).await? == 0 {
            return Ok(BackendEvent::Disconnected);
        }
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.last_sequence = e.sequence;
        Ok(Self::event(e.message))
    }
}
impl Drop for BridgeBackend {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ansi_and_severity() {
        assert_eq!(strip_ansi("\x1b[31merror: bad\x1b[0m"), "error: bad");
        assert_eq!(
            classify_output("WARNING: x".into()).severity,
            Severity::Warning
        )
    }
}
