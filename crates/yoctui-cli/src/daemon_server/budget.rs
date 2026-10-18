use super::*;
use yoctui_protocol::daemon::DaemonSnapshotError;

#[cfg(test)]
#[path = "../tests/daemon_budget.rs"]
mod tests;

pub(super) fn is_size_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<DaemonSnapshotError>(),
        Some(
            DaemonSnapshotError::SnapshotTooLarge { .. }
                | DaemonSnapshotError::EventTooLarge { .. }
                | DaemonSnapshotError::TooManyRawExecutions
        )
    )
}

/// A rejected payload is not a daemon-lifetime failure. Keep the last valid
/// checkpoint/replay cursor and every client, build and PTY alive. Do not mask
/// malformed state or unrelated internal errors.
pub(super) fn recover(result: Result<()>, journal: &mut DaemonSnapshotJournal) -> Result<()> {
    match result {
        Err(error) if is_size_error(&error) => {
            report(journal, &error);
            Ok(())
        }
        other => other,
    }
}

pub(super) fn report(journal: &mut DaemonSnapshotJournal, error: &anyhow::Error) {
    let message = format!(
        "Oversized daemon update rejected; daemon and client connections remain active: {error}"
    );
    eprintln!("{} daemon-budget: {message}", unix_ms());
    // This tiny diagnostic is best-effort; a full checkpoint must never make
    // reporting a rejected payload fail recursively.
    let _ = journal.publish(yoctui_protocol::daemon::DaemonEvent::Log(
        yoctui_protocol::daemon::LogRecord {
            source: "daemon-budget".into(),
            severity: yoctui_protocol::daemon::LogSeverity::Warning,
            message,
            unix_ms: unix_ms(),
            recipe: None,
            task: None,
            path: None,
            build: None,
        },
    ));
}

pub(super) fn send_command_result(
    connection: &mut DaemonConnection,
    journal: &mut DaemonSnapshotJournal,
    result: CommandResult,
) -> Result<(), IpcError> {
    let request_id = result.request_id;
    match connection.send(&ServerMessage::CommandResult(result)) {
        Err(IpcError::Protocol(yoctui_protocol::daemon::DaemonProtocolError::TooLarge)) => {
            // Encoding failed before writing any bytes, so the socket is still
            // usable. Reply to the same request rather than dropping the peer.
            connection.send(&ServerMessage::CommandResult(CommandResult {
                request_id,
                outcome: CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::MessageTooLarge,
                    message:
                        "Response exceeds the transport budget; daemon connection remains active"
                            .into(),
                    current_generation: journal.snapshot().generation,
                },
            }))
        }
        other => other,
    }
}
