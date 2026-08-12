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
    pub lifecycle: PtyAttachLifecycle,
    pub dimensions: PtyDimensions,
    pub viewers: usize,
    pub writer: Option<PtyClientId>,
    pub writer_epoch: u64,
    pub exit_status: Option<PtyExitStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PtyAttachSnapshot {
    pub listing: PtyAttachListing,
    pub terminal: TerminalSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PtyAttachEvent {
    Started,
    Output { sequence: u64 },
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
            lifecycle: listing_lifecycle(session.lifecycle),
            dimensions: session.dimensions,
            viewers: session.attached_clients.len(),
            writer: session.writer.map(|writer| writer.client),
            writer_epoch: session.writer_epoch,
            exit_status: session.exit_status,
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
                Ok(PtyAttachEvent::Output { sequence })
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
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{PtyCommandIdentity, PtySessionId, PtyWorkspaceContext};

    static FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn fixture() -> (PathBuf, PtySessionSpec) {
        let id = FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("yoctui-pty-attach-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("build")).unwrap();
        let spec = PtySessionSpec {
            id: PtySessionId(id),
            name: "persistent shell".into(),
            kind: PtySessionKind::BuildShell,
            cwd: root.join("build"),
            command: PtyCommandIdentity {
                executable: "/bin/sh".into(),
                arguments: vec![
                    "-c".into(),
                    "printf 'ready\\n'; while IFS= read -r line; do [ \"$line\" = exit ] && exit 0; printf 'seen:%s\\n' \"$line\"; done".into(),
                ],
            },
            dimensions: PtyDimensions {
                columns: 80,
                rows: 24,
            },
            restartable: true,
            workspace: PtyWorkspaceContext {
                source_dir: root.clone(),
                build_dir: root.join("build"),
                owner_identity: format!("fixture-{id}"),
            },
        };
        (root, spec)
    }

    async fn pump_until(session: &mut DaemonPtySession, needle: &str) {
        loop {
            match session.next_event().await.unwrap() {
                PtyAttachEvent::Output { .. } => {
                    if session
                        .snapshot(0)
                        .unwrap()
                        .terminal
                        .plain_text
                        .contains(needle)
                    {
                        return;
                    }
                }
                event => assert!(!matches!(event, PtyAttachEvent::Lost { .. })),
            }
        }
    }

    #[tokio::test]
    async fn pty_attach_prefix_detach_and_client_exit_leave_process_running() {
        let (root, spec) = fixture();
        let mut session = DaemonPtySession::start(
            spec,
            BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            100,
            Duration::from_secs(1),
        )
        .await
        .unwrap();
        let first = PtyClientId([1; 16]);
        let initial = session.attach(first).unwrap();
        assert_eq!(initial.listing.lifecycle, PtyAttachLifecycle::Running);
        assert_eq!(initial.listing.viewers, 1);
        let epoch = session.take_control(first, 0).unwrap();
        session
            .resize(
                first,
                epoch,
                PtyDimensions {
                    columns: 100,
                    rows: 30,
                },
            )
            .unwrap();
        pump_until(&mut session, "ready").await;
        session
            .input(first, epoch, b"before-detach\n")
            .await
            .unwrap();
        pump_until(&mut session, "seen:before-detach").await;

        session.prefix_return(first).unwrap();
        assert!(session.is_process_active());
        assert_eq!(session.listing().unwrap().viewers, 0);
        assert_eq!(session.listing().unwrap().writer, None);

        let second = PtyClientId([2; 16]);
        let restored = session.attach(second).unwrap();
        assert!(restored.terminal.plain_text.contains("seen:before-detach"));
        assert_eq!(
            restored.terminal.dimensions,
            PtyDimensions {
                columns: 100,
                rows: 30
            }
        );
        assert_eq!(restored.listing.viewers, 1);
        let epoch = session
            .take_control(second, restored.listing.writer_epoch)
            .unwrap();
        session.client_disconnected(second).unwrap();
        assert!(session.is_process_active());
        assert_eq!(session.listing().unwrap().writer_epoch, epoch + 1);

        let third = PtyClientId([3; 16]);
        let restored = session.attach(third).unwrap();
        let epoch = session
            .take_control(third, restored.listing.writer_epoch)
            .unwrap();
        session.input(third, epoch, b"exit\n").await.unwrap();
        loop {
            if matches!(
                session.next_event().await.unwrap(),
                PtyAttachEvent::Exited(PtyExitStatus::Code(0))
            ) {
                break;
            }
        }
        assert_eq!(
            session.listing().unwrap().lifecycle,
            PtyAttachLifecycle::Exited
        );
        assert!(!session.is_process_active());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_attach_listing_distinguishes_running_exited_and_lost() {
        assert_eq!(
            listing_lifecycle(PtySessionLifecycle::Running),
            PtyAttachLifecycle::Running
        );
        assert_eq!(
            listing_lifecycle(PtySessionLifecycle::Exited),
            PtyAttachLifecycle::Exited
        );
        assert_eq!(
            listing_lifecycle(PtySessionLifecycle::Lost),
            PtyAttachLifecycle::Lost
        );
    }
}
