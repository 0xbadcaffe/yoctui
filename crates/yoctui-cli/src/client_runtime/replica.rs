use std::time::{Duration, Instant};

use yoctui_model::App;
use yoctui_protocol::daemon::{CommandOutcome, CommandResult};

use crate::client_transport::ClientServerEvent;

use super::{
    ClientRuntimeError, InteractiveDaemonRuntime, MAX_EVENTS_PER_POLL, MAX_POLL_DURATION,
    TerminalCompletionOutcome, attach::restore_local_build_dir,
};

impl InteractiveDaemonRuntime {
    pub fn poll(&mut self, app: &mut App) -> Result<bool, ClientRuntimeError> {
        let mut received = false;
        let mut pending_logs = Vec::new();
        let mut pending_tasks = Vec::new();
        let started = Instant::now();
        for _ in 0..MAX_EVENTS_PER_POLL {
            let Some(event) = self.transport.try_receive(Duration::from_millis(1))? else {
                break;
            };
            received = true;
            if matches!(
                &event,
                ClientServerEvent::Event(yoctui_protocol::daemon::SequencedEvent {
                    event: yoctui_protocol::daemon::DaemonEvent::Log(_),
                    ..
                })
            ) {
                let ClientServerEvent::Event(event) = event else {
                    unreachable!("the guarded event is a daemon event")
                };
                self.flush_task_events(app, &mut pending_tasks)?;
                pending_logs.push(event);
                if started.elapsed() >= MAX_POLL_DURATION {
                    break;
                }
                continue;
            }
            if matches!(
                &event,
                ClientServerEvent::Event(yoctui_protocol::daemon::SequencedEvent {
                    event: yoctui_protocol::daemon::DaemonEvent::Build(
                        yoctui_protocol::daemon::DaemonBuildEvent::TaskQueued { .. }
                            | yoctui_protocol::daemon::DaemonBuildEvent::TaskStarted { .. }
                            | yoctui_protocol::daemon::DaemonBuildEvent::TaskProgress { .. }
                            | yoctui_protocol::daemon::DaemonBuildEvent::TaskCompleted { .. }
                    ),
                    ..
                })
            ) {
                let ClientServerEvent::Event(event) = event else {
                    unreachable!("the guarded event is a daemon event")
                };
                self.flush_log_events(app, &mut pending_logs)?;
                pending_tasks.push(event);
                if started.elapsed() >= MAX_POLL_DURATION {
                    break;
                }
                continue;
            }
            self.flush_log_events(app, &mut pending_logs)?;
            self.flush_task_events(app, &mut pending_tasks)?;
            match event {
                ClientServerEvent::Snapshot(snapshot) => self.replica.replace_app(app, *snapshot),
                ClientServerEvent::Event(event) => self.replica.apply_event_to_app(app, &event)?,
                ClientServerEvent::ResyncRequired { reason, .. } => {
                    self.replica.begin_synchronization();
                    self.replica.install_app(app);
                    app.notification = Some(format!("Daemon resynchronization required: {reason}"));
                }
                ClientServerEvent::CommandResult(result) => {
                    if matches!(result.outcome, CommandOutcome::Rejected { .. }) {
                        app.cancel_pending_platform_menuconfig();
                    }
                    app.notification = command_result_notification(result);
                }
                ClientServerEvent::ShuttingDown => {
                    self.replica.disconnect_app(app);
                    app.notification = Some("Yoctui daemon is shutting down.".into());
                }
            }
            restore_local_build_dir(app, self.local_build_dir.as_ref());
            if started.elapsed() >= MAX_POLL_DURATION {
                break;
            }
        }
        self.flush_log_events(app, &mut pending_logs)?;
        self.flush_task_events(app, &mut pending_tasks)?;
        self.reconcile_terminal_completions(app);
        Ok(received)
    }

    fn reconcile_terminal_completions(&mut self, app: &App) {
        let mut index = 0;
        while index < self.pending_terminal_completions.len() {
            let pending = &mut self.pending_terminal_completions[index];
            if pending.session_id.is_none() {
                pending.session_id = app
                    .daemon
                    .pty_sessions
                    .iter()
                    .find(|session| {
                        session.name == pending.name
                            && !pending.known_sessions.contains(&session.id)
                    })
                    .map(|session| session.id);
            }
            let Some(session_id) = pending.session_id else {
                index += 1;
                continue;
            };
            let outcome = terminal_completion_outcome(app, pending, session_id);
            if let Some(outcome) = outcome {
                self.terminal_completions.push(outcome);
                self.pending_terminal_completions.remove(index);
            } else {
                index += 1;
            }
        }
    }

    pub(crate) fn take_terminal_completions(&mut self) -> Vec<TerminalCompletionOutcome> {
        std::mem::take(&mut self.terminal_completions)
    }

    fn flush_log_events(
        &mut self,
        app: &mut App,
        pending: &mut Vec<yoctui_protocol::daemon::SequencedEvent>,
    ) -> Result<(), ClientRuntimeError> {
        if pending.is_empty() {
            return Ok(());
        }
        self.replica.apply_log_events_to_app(app, pending)?;
        pending.clear();
        restore_local_build_dir(app, self.local_build_dir.as_ref());
        Ok(())
    }

    fn flush_task_events(
        &mut self,
        app: &mut App,
        pending: &mut Vec<yoctui_protocol::daemon::SequencedEvent>,
    ) -> Result<(), ClientRuntimeError> {
        if pending.is_empty() {
            return Ok(());
        }
        self.replica.apply_task_events_to_app(app, pending)?;
        pending.clear();
        restore_local_build_dir(app, self.local_build_dir.as_ref());
        Ok(())
    }
}

pub(super) fn terminal_completion_outcome(
    app: &App,
    pending: &super::PendingTerminalCompletion,
    session_id: u64,
) -> Option<TerminalCompletionOutcome> {
    let summary = app
        .daemon
        .pty_sessions
        .iter()
        .find(|session| session.id == session_id)?;
    let details = app
        .daemon
        .pty_details
        .iter()
        .find(|details| details.id == session_id);
    match summary.lifecycle {
        yoctui_model::ClientDaemonLifecycle::Exited => match details.and_then(|d| d.exit_code) {
            Some(0) => Some(TerminalCompletionOutcome::Succeeded(
                pending.completion.clone(),
            )),
            Some(code) => Some(TerminalCompletionOutcome::Failed(format!(
                "{} exited with status {code}; the generated file was not opened.",
                pending.name
            ))),
            None => None,
        },
        yoctui_model::ClientDaemonLifecycle::Failed | yoctui_model::ClientDaemonLifecycle::Lost => {
            Some(TerminalCompletionOutcome::Failed(format!(
                "{} failed; the generated file was not opened.",
                pending.name
            )))
        }
        _ => None,
    }
}

pub(crate) fn command_result_notification(result: CommandResult) -> Option<String> {
    match result.outcome {
        CommandOutcome::Accepted => None,
        CommandOutcome::Completed => {
            Some(format!("Daemon request {} completed.", result.request_id.0))
        }
        CommandOutcome::Rejected { message, .. } => Some(format!(
            "Daemon request {} was rejected: {message}",
            result.request_id.0
        )),
        outcome => Some(format!(
            "Daemon request {}: {outcome:?}",
            result.request_id.0
        )),
    }
}
