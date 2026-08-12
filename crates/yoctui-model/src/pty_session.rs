use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
};

use thiserror::Error;

pub const MIN_PTY_COLUMNS: u16 = 2;
pub const MIN_PTY_ROWS: u16 = 1;
pub const MAX_PTY_COLUMNS: u16 = 1_000;
pub const MAX_PTY_ROWS: u16 = 1_000;
pub const MAX_PTY_NAME_BYTES: usize = 128;
pub const MAX_PTY_ARGUMENTS: usize = 128;
pub const MAX_PTY_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_PTY_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_PTY_SCROLLBACK_CELLS: usize = 10_000_000;
pub const MAX_PTY_SCROLLBACK_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PtySessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PtyClientId(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyDimensions {
    pub columns: u16,
    pub rows: u16,
}

impl PtyDimensions {
    pub fn validate(self) -> Result<Self, PtySessionError> {
        if !(MIN_PTY_COLUMNS..=MAX_PTY_COLUMNS).contains(&self.columns)
            || !(MIN_PTY_ROWS..=MAX_PTY_ROWS).contains(&self.rows)
        {
            return Err(PtySessionError::InvalidDimensions(self));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionKind {
    BuildShell,
    SourceShell,
    LayerShell,
    RecipeShell,
    DevtoolShell,
    SdkShell,
    DeployShell,
    Devshell,
    Menuconfig,
    InteractiveTool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCommandIdentity {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyWorkspaceContext {
    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub owner_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySessionSpec {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub dimensions: PtyDimensions,
    pub restartable: bool,
    pub workspace: PtyWorkspaceContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionLifecycle {
    Starting,
    Running,
    Terminating,
    Exited,
    Lost,
}

impl PtySessionLifecycle {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Exited | Self::Lost)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyExitStatus {
    Code(i32),
    Signal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PtyScrollbackMetadata {
    pub first_sequence: u64,
    pub next_sequence: u64,
    pub retained_lines: usize,
    pub retained_cells: usize,
    pub retained_bytes: usize,
    pub dropped_lines: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyWriterLease {
    pub client: PtyClientId,
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySession {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub lifecycle: PtySessionLifecycle,
    pub dimensions: PtyDimensions,
    pub attached_clients: BTreeSet<PtyClientId>,
    pub writer: Option<PtyWriterLease>,
    pub writer_epoch: u64,
    pub process_group: Option<i32>,
    pub scrollback: PtyScrollbackMetadata,
    pub exit_status: Option<PtyExitStatus>,
    pub restartable: bool,
    pub workspace: PtyWorkspaceContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtySessionAction {
    MarkRunning,
    Attach(PtyClientId),
    Detach(PtyClientId),
    TakeControl {
        client: PtyClientId,
        expected_epoch: u64,
    },
    ReleaseControl {
        client: PtyClientId,
        expected_epoch: u64,
    },
    Resize {
        client: PtyClientId,
        writer_epoch: u64,
        dimensions: PtyDimensions,
    },
    AdvanceScrollback(PtyScrollbackMetadata),
    BeginTermination,
    Exit(PtyExitStatus),
    MarkLost,
    Rename(String),
}

impl PtySession {
    pub fn new(spec: PtySessionSpec, process_group: i32) -> Result<Self, PtySessionError> {
        validate_spec(&spec)?;
        if process_group <= 0 {
            return Err(PtySessionError::InvalidProcessGroup(process_group));
        }
        Ok(Self {
            id: spec.id,
            name: spec.name,
            kind: spec.kind,
            cwd: spec.cwd,
            command: spec.command,
            lifecycle: PtySessionLifecycle::Starting,
            dimensions: spec.dimensions,
            attached_clients: BTreeSet::new(),
            writer: None,
            writer_epoch: 0,
            process_group: Some(process_group),
            scrollback: PtyScrollbackMetadata::default(),
            exit_status: None,
            restartable: spec.restartable,
            workspace: spec.workspace,
        })
    }

    pub fn apply(&mut self, action: PtySessionAction) -> Result<(), PtySessionError> {
        match action {
            PtySessionAction::MarkRunning => {
                self.require(PtySessionLifecycle::Starting)?;
                self.lifecycle = PtySessionLifecycle::Running;
            }
            PtySessionAction::Attach(client) => {
                self.attached_clients.insert(client);
            }
            PtySessionAction::Detach(client) => {
                if !self.attached_clients.remove(&client) {
                    return Err(PtySessionError::ClientNotAttached(client));
                }
                if self.writer.is_some_and(|writer| writer.client == client) {
                    self.release_writer()?;
                }
            }
            PtySessionAction::TakeControl {
                client,
                expected_epoch,
            } => {
                if !self.attached_clients.contains(&client) {
                    return Err(PtySessionError::ClientNotAttached(client));
                }
                if self.lifecycle != PtySessionLifecycle::Running {
                    return Err(PtySessionError::NotRunning(self.lifecycle));
                }
                if self.writer.is_some() {
                    return Err(PtySessionError::WriterBusy);
                }
                if expected_epoch != self.writer_epoch {
                    return Err(PtySessionError::StaleWriterEpoch {
                        expected: self.writer_epoch,
                        actual: expected_epoch,
                    });
                }
                self.writer_epoch = self
                    .writer_epoch
                    .checked_add(1)
                    .ok_or(PtySessionError::WriterEpochExhausted)?;
                self.writer = Some(PtyWriterLease {
                    client,
                    epoch: self.writer_epoch,
                });
            }
            PtySessionAction::ReleaseControl {
                client,
                expected_epoch,
            } => self
                .require_writer(client, expected_epoch)
                .and_then(|_| self.release_writer())?,
            PtySessionAction::Resize {
                client,
                writer_epoch,
                dimensions,
            } => {
                self.require_writer(client, writer_epoch)?;
                self.dimensions = dimensions.validate()?;
            }
            PtySessionAction::AdvanceScrollback(next) => {
                if next.next_sequence < self.scrollback.next_sequence
                    || next.first_sequence > next.next_sequence
                    || next.retained_lines > next.retained_cells
                    || next.retained_cells > next.retained_bytes.saturating_mul(4)
                    || next.retained_lines > MAX_PTY_SCROLLBACK_LINES
                    || next.retained_cells > MAX_PTY_SCROLLBACK_CELLS
                    || next.retained_bytes > MAX_PTY_SCROLLBACK_BYTES
                    || next.dropped_lines < self.scrollback.dropped_lines
                {
                    return Err(PtySessionError::InvalidScrollback);
                }
                self.scrollback = next;
            }
            PtySessionAction::BeginTermination => {
                self.require(PtySessionLifecycle::Running)?;
                self.lifecycle = PtySessionLifecycle::Terminating;
            }
            PtySessionAction::Exit(status) => {
                if !matches!(
                    self.lifecycle,
                    PtySessionLifecycle::Starting
                        | PtySessionLifecycle::Running
                        | PtySessionLifecycle::Terminating
                ) {
                    return Err(PtySessionError::InvalidTransition(self.lifecycle));
                }
                self.lifecycle = PtySessionLifecycle::Exited;
                self.exit_status = Some(status);
                self.clear_live_ownership()?;
            }
            PtySessionAction::MarkLost => {
                if self.lifecycle.is_terminal() {
                    return Err(PtySessionError::InvalidTransition(self.lifecycle));
                }
                self.lifecycle = PtySessionLifecycle::Lost;
                self.exit_status = None;
                self.clear_live_ownership()?;
            }
            PtySessionAction::Rename(name) => {
                validate_name(&name)?;
                self.name = name;
            }
        }
        Ok(())
    }

    fn require(&self, lifecycle: PtySessionLifecycle) -> Result<(), PtySessionError> {
        if self.lifecycle != lifecycle {
            return Err(PtySessionError::InvalidTransition(self.lifecycle));
        }
        Ok(())
    }

    fn require_writer(&self, client: PtyClientId, epoch: u64) -> Result<(), PtySessionError> {
        if self.writer != Some(PtyWriterLease { client, epoch }) {
            return Err(PtySessionError::NotWriter);
        }
        Ok(())
    }

    fn release_writer(&mut self) -> Result<(), PtySessionError> {
        self.writer = None;
        self.writer_epoch = self
            .writer_epoch
            .checked_add(1)
            .ok_or(PtySessionError::WriterEpochExhausted)?;
        Ok(())
    }

    fn clear_live_ownership(&mut self) -> Result<(), PtySessionError> {
        if self.writer.is_some() {
            self.release_writer()?;
        }
        self.attached_clients.clear();
        self.process_group = None;
        Ok(())
    }
}

fn validate_spec(spec: &PtySessionSpec) -> Result<(), PtySessionError> {
    if spec.id.0 == 0 {
        return Err(PtySessionError::InvalidSessionId);
    }
    validate_name(&spec.name)?;
    spec.dimensions.validate()?;
    for path in [
        &spec.cwd,
        &spec.command.executable,
        &spec.workspace.source_dir,
        &spec.workspace.build_dir,
    ] {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(PtySessionError::InvalidPath(path.clone()));
        }
    }
    if !spec.cwd.starts_with(&spec.workspace.source_dir)
        && !spec.cwd.starts_with(&spec.workspace.build_dir)
    {
        return Err(PtySessionError::CwdOutsideWorkspace);
    }
    if spec.workspace.owner_identity.trim().is_empty()
        || spec.workspace.owner_identity.len() > 512
        || spec.workspace.owner_identity.chars().any(char::is_control)
    {
        return Err(PtySessionError::InvalidWorkspaceIdentity);
    }
    if spec.command.arguments.len() > MAX_PTY_ARGUMENTS
        || spec
            .command
            .arguments
            .iter()
            .map(String::len)
            .sum::<usize>()
            > MAX_PTY_ARGUMENT_BYTES
        || spec
            .command
            .arguments
            .iter()
            .any(|argument| argument.contains('\0'))
    {
        return Err(PtySessionError::InvalidCommand);
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), PtySessionError> {
    if name.trim().is_empty()
        || name.len() > MAX_PTY_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(PtySessionError::InvalidName);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PtySessionError {
    #[error("PTY session ID must be nonzero")]
    InvalidSessionId,
    #[error("PTY session name is empty, oversized, or contains controls")]
    InvalidName,
    #[error("invalid PTY dimensions: {0:?}")]
    InvalidDimensions(PtyDimensions),
    #[error("invalid PTY path: {0}")]
    InvalidPath(PathBuf),
    #[error("PTY working directory is outside its workspace")]
    CwdOutsideWorkspace,
    #[error("invalid PTY workspace identity")]
    InvalidWorkspaceIdentity,
    #[error("PTY command arguments are invalid or oversized")]
    InvalidCommand,
    #[error("invalid PTY process group: {0}")]
    InvalidProcessGroup(i32),
    #[error("invalid PTY transition from {0:?}")]
    InvalidTransition(PtySessionLifecycle),
    #[error("PTY is not running: {0:?}")]
    NotRunning(PtySessionLifecycle),
    #[error("PTY client is not attached: {0:?}")]
    ClientNotAttached(PtyClientId),
    #[error("PTY already has a writer")]
    WriterBusy,
    #[error("PTY action does not own the writer lease")]
    NotWriter,
    #[error("stale PTY writer epoch {actual}; expected {expected}")]
    StaleWriterEpoch { expected: u64, actual: u64 },
    #[error("PTY writer epoch is exhausted")]
    WriterEpochExhausted,
    #[error("invalid PTY scrollback metadata")]
    InvalidScrollback,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> PtySessionSpec {
        PtySessionSpec {
            id: PtySessionId(7),
            name: "kernel menuconfig".into(),
            kind: PtySessionKind::Menuconfig,
            cwd: "/work/poky/build".into(),
            command: PtyCommandIdentity {
                executable: "/work/poky/bitbake/bin/bitbake".into(),
                arguments: vec!["-c".into(), "menuconfig".into(), "virtual/kernel".into()],
            },
            dimensions: PtyDimensions {
                columns: 120,
                rows: 40,
            },
            restartable: true,
            workspace: PtyWorkspaceContext {
                source_dir: "/work/poky".into(),
                build_dir: "/work/poky/build".into(),
                owner_identity: "workspace-1".into(),
            },
        }
    }

    fn client(value: u8) -> PtyClientId {
        PtyClientId([value; 16])
    }

    #[test]
    fn pty_session_validates_identity_context_and_dimensions() {
        let session = PtySession::new(spec(), 431).unwrap();
        assert_eq!(session.lifecycle, PtySessionLifecycle::Starting);
        let mut invalid = spec();
        invalid.cwd = "/tmp".into();
        assert_eq!(
            PtySession::new(invalid, 431),
            Err(PtySessionError::CwdOutsideWorkspace)
        );
        let mut invalid = spec();
        invalid.dimensions.columns = 0;
        assert!(matches!(
            PtySession::new(invalid, 431),
            Err(PtySessionError::InvalidDimensions(_))
        ));
    }

    #[test]
    fn pty_session_enforces_single_writer_epochs_resize_and_detach() {
        let mut session = PtySession::new(spec(), 431).unwrap();
        session.apply(PtySessionAction::MarkRunning).unwrap();
        session.apply(PtySessionAction::Attach(client(1))).unwrap();
        session.apply(PtySessionAction::Attach(client(2))).unwrap();
        session
            .apply(PtySessionAction::TakeControl {
                client: client(1),
                expected_epoch: 0,
            })
            .unwrap();
        assert_eq!(session.writer.unwrap().epoch, 1);
        assert_eq!(
            session.apply(PtySessionAction::TakeControl {
                client: client(2),
                expected_epoch: 1,
            }),
            Err(PtySessionError::WriterBusy)
        );
        assert_eq!(
            session.apply(PtySessionAction::Resize {
                client: client(2),
                writer_epoch: 1,
                dimensions: PtyDimensions {
                    columns: 80,
                    rows: 24
                },
            }),
            Err(PtySessionError::NotWriter)
        );
        session.apply(PtySessionAction::Detach(client(1))).unwrap();
        assert_eq!(session.writer, None);
        assert_eq!(session.writer_epoch, 2);
        assert_eq!(session.attached_clients, BTreeSet::from([client(2)]));
    }

    #[test]
    fn pty_session_bounds_scrollback_and_clears_live_ownership_on_exit() {
        let mut session = PtySession::new(spec(), 431).unwrap();
        session.apply(PtySessionAction::MarkRunning).unwrap();
        session.apply(PtySessionAction::Attach(client(1))).unwrap();
        session
            .apply(PtySessionAction::TakeControl {
                client: client(1),
                expected_epoch: 0,
            })
            .unwrap();
        session
            .apply(PtySessionAction::AdvanceScrollback(PtyScrollbackMetadata {
                first_sequence: 3,
                next_sequence: 10,
                retained_lines: 4,
                retained_cells: 20,
                retained_bytes: 20,
                dropped_lines: 3,
            }))
            .unwrap();
        session
            .apply(PtySessionAction::Exit(PtyExitStatus::Code(0)))
            .unwrap();
        assert_eq!(session.lifecycle, PtySessionLifecycle::Exited);
        assert_eq!(session.process_group, None);
        assert!(session.attached_clients.is_empty());
        assert_eq!(session.writer, None);
        assert_eq!(session.exit_status, Some(PtyExitStatus::Code(0)));
        assert_eq!(
            session.apply(PtySessionAction::MarkLost),
            Err(PtySessionError::InvalidTransition(
                PtySessionLifecycle::Exited
            ))
        );
    }
}
