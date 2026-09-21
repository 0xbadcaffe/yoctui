use std::{collections::BTreeMap, time::Duration};

use yoctui_bitbake::{PtyRunner, PtyRunnerError, PtyRunnerEvent};
use yoctui_model::{
    MAX_TERMINAL_SNAPSHOT_CELLS, PtyClientId, PtyDimensions, PtyExitStatus, PtySessionAction,
    PtySessionKind, PtySessionLifecycle, PtySessionSpec, TerminalEmulationError, TerminalEmulator,
    TerminalSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PtyAttachLifecycle {
    Running,
    Exited,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PtyAttachListing {
    pub id: yoctui_model::PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: std::path::PathBuf,
    pub lifecycle: PtyAttachLifecycle,
    pub dimensions: PtyDimensions,
    pub viewers: usize,
    pub writer: Option<PtyClientId>,
    pub writer_epoch: u64,
    pub exit_status: Option<PtyExitStatus>,
    pub restartable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PtyAttachSnapshot {
    pub listing: PtyAttachListing,
    pub terminal: TerminalSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PtyAttachEvent {
    Started,
    Output { sequence: u64, bytes: Vec<u8> },
    Exited(PtyExitStatus),
    Lost { message: String },
}

pub(crate) struct DaemonPtySession {
    runner: PtyRunner,
    emulator: TerminalEmulator,
}

impl DaemonPtySession {
    pub async fn start(
        spec: PtySessionSpec,
        environment: BTreeMap<String, String>,
        scrollback_lines: usize,
        termination_grace: Duration,
    ) -> Result<Self, PtyAttachError> {
        let emulator = TerminalEmulator::new(spec.dimensions, scrollback_lines)?;
        let mut runner = PtyRunner::default().with_termination_grace(termination_grace);
        runner.start(spec, environment).await?;
        Ok(Self { runner, emulator })
    }

    pub fn listing(&self) -> Result<PtyAttachListing, PtyAttachError> {
        let session = self
            .runner
            .session()
            .ok_or(PtyAttachError::SessionMissing)?;
        Ok(PtyAttachListing {
            id: session.id,
            name: session.name.clone(),
            kind: session.kind,
            cwd: session.cwd.clone(),
            lifecycle: listing_lifecycle(session.lifecycle),
            dimensions: session.dimensions,
            viewers: session.attached_clients.len(),
            writer: session.writer.map(|writer| writer.client),
            writer_epoch: session.writer_epoch,
            exit_status: session.exit_status,
            restartable: session.restartable,
        })
    }

    pub fn attach(&mut self, client: PtyClientId) -> Result<PtyAttachSnapshot, PtyAttachError> {
        self.runner
            .apply_session_action(PtySessionAction::Attach(client))?;
        self.snapshot(0)
    }

    pub fn detach(&mut self, client: PtyClientId) -> Result<(), PtyAttachError> {
        self.runner
            .apply_session_action(PtySessionAction::Detach(client))?;
        Ok(())
    }

    pub fn prefix_return(&mut self, client: PtyClientId) -> Result<(), PtyAttachError> {
        self.detach(client)
    }

    pub fn client_disconnected(&mut self, client: PtyClientId) -> Result<(), PtyAttachError> {
        let attached = self
            .runner
            .session()
            .is_some_and(|session| session.attached_clients.contains(&client));
        if attached {
            self.detach(client)?;
        }
        Ok(())
    }

    pub fn take_control(
        &mut self,
        client: PtyClientId,
        expected_epoch: u64,
    ) -> Result<u64, PtyAttachError> {
        self.runner
            .apply_session_action(PtySessionAction::TakeControl {
                client,
                expected_epoch,
            })?;
        Ok(self.listing()?.writer_epoch)
    }

    pub fn release_control(
        &mut self,
        client: PtyClientId,
        expected_epoch: u64,
    ) -> Result<(), PtyAttachError> {
        self.runner
            .apply_session_action(PtySessionAction::ReleaseControl {
                client,
                expected_epoch,
            })?;
        Ok(())
    }

    pub fn rename(&mut self, name: String) -> Result<(), PtyAttachError> {
        self.runner
            .apply_session_action(PtySessionAction::Rename(name))?;
        Ok(())
    }

    pub async fn input(
        &mut self,
        client: PtyClientId,
        writer_epoch: u64,
        bytes: &[u8],
    ) -> Result<(), PtyAttachError> {
        self.runner.input(client, writer_epoch, bytes).await?;
        Ok(())
    }

    pub fn resize(
        &mut self,
        client: PtyClientId,
        writer_epoch: u64,
        dimensions: PtyDimensions,
    ) -> Result<(), PtyAttachError> {
        let cells = usize::from(dimensions.rows) * usize::from(dimensions.columns);
        if cells > MAX_TERMINAL_SNAPSHOT_CELLS {
            return Err(TerminalEmulationError::ScreenTooLarge {
                cells,
                maximum: MAX_TERMINAL_SNAPSHOT_CELLS,
            }
            .into());
        }
        self.runner.resize(client, writer_epoch, dimensions)?;
        self.emulator.resize(dimensions)?;
        Ok(())
    }

    pub async fn terminate(&mut self) -> Result<bool, PtyAttachError> {
        Ok(self.runner.terminate().await?)
    }

    pub fn snapshot(
        &mut self,
        scrollback_offset: usize,
    ) -> Result<PtyAttachSnapshot, PtyAttachError> {
        Ok(PtyAttachSnapshot {
            listing: self.listing()?,
            terminal: self.emulator.snapshot(scrollback_offset)?,
        })
    }

    pub async fn next_event(&mut self) -> Result<PtyAttachEvent, PtyAttachError> {
        match self.runner.next_event().await? {
            PtyRunnerEvent::Started => Ok(PtyAttachEvent::Started),
            PtyRunnerEvent::Output { sequence, bytes } => {
                self.emulator.process(&bytes)?;
                Ok(PtyAttachEvent::Output { sequence, bytes })
            }
            PtyRunnerEvent::Exited(status) => Ok(PtyAttachEvent::Exited(status)),
            PtyRunnerEvent::Lost { message } => Ok(PtyAttachEvent::Lost { message }),
        }
    }

    pub fn is_process_active(&self) -> bool {
        self.runner.is_active()
    }
}

fn listing_lifecycle(lifecycle: PtySessionLifecycle) -> PtyAttachLifecycle {
    match lifecycle {
        PtySessionLifecycle::Starting
        | PtySessionLifecycle::Running
        | PtySessionLifecycle::Terminating => PtyAttachLifecycle::Running,
        PtySessionLifecycle::Exited => PtyAttachLifecycle::Exited,
        PtySessionLifecycle::Lost => PtyAttachLifecycle::Lost,
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PtyAttachError {
    #[error(transparent)]
    Runner(#[from] PtyRunnerError),
    #[error(transparent)]
    Emulator(#[from] TerminalEmulationError),
    #[error("daemon PTY session state is unavailable")]
    SessionMissing,
}

#[cfg(test)]
#[path = "tests/pty_attach/mod.rs"]
mod tests;
