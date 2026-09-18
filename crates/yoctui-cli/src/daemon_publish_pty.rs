//! Daemon publish pty.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_pty_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_pty::DaemonPtyEvent,
) -> Result<()> {
    use daemon_pty::DaemonPtyEvent;
    use yoctui_protocol::daemon::DaemonEvent;
    match event {
        DaemonPtyEvent::Started {
            session_id,
            snapshot,
        }
        | DaemonPtyEvent::Changed {
            session_id,
            snapshot,
        } => {
            let _ = session_id;
            journal.publish(DaemonEvent::PtyChanged(snapshot))?;
        }
        DaemonPtyEvent::Output {
            session_id,
            bytes,
            screen,
        } => {
            journal.publish(DaemonEvent::PtyOutput {
                session_id: yoctui_protocol::daemon::PtySessionId(session_id.0),
                bytes,
            })?;
            if let Some(screen) = screen
                && let Err(error) = journal.publish(DaemonEvent::PtyScreen(screen))
            {
                tracing::warn!(%error, "discarding invalid or oversized PTY screen snapshot");
            }
        }
        DaemonPtyEvent::Exited {
            session_id,
            exit_code,
            screen,
        } => {
            if let Some(screen) = screen
                && let Err(error) = journal.publish(DaemonEvent::PtyScreen(screen))
            {
                tracing::warn!(%error, "discarding invalid or oversized final PTY screen snapshot");
            }
            if let Some(existing) = journal
                .snapshot()
                .pty_sessions
                .iter()
                .find(|session| session.id.0 == session_id.0)
                .cloned()
            {
                journal.publish(DaemonEvent::PtyChanged(
                    yoctui_protocol::daemon::PtySessionSummary {
                        lifecycle: yoctui_protocol::daemon::LifecycleState::Exited,
                        exit_code,
                        ..existing
                    },
                ))?;
            }
        }
        DaemonPtyEvent::Lost {
            session_id,
            message,
        } => {
            if let Some(existing) = journal
                .snapshot()
                .pty_sessions
                .iter()
                .find(|session| session.id.0 == session_id.0)
                .cloned()
            {
                journal.publish(DaemonEvent::PtyChanged(
                    yoctui_protocol::daemon::PtySessionSummary {
                        lifecycle: yoctui_protocol::daemon::LifecycleState::Lost,
                        name: format!("{}: {message}", existing.name),
                        ..existing
                    },
                ))?;
            }
        }
    }
    Ok(())
}
