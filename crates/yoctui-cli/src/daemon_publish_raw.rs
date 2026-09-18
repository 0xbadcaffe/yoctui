//! Daemon publish raw.
use super::*;

pub(crate) fn publish_daemon_raw_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_raw::DaemonRawEvent,
) -> Result<()> {
    use yoctui_protocol::daemon::{DaemonEvent, JobKind, JobSummary};
    let command = event.state.request.command.to_string();
    journal.publish(DaemonEvent::RawExecutionChanged(Box::new(
        yoctui_app::raw_execution_snapshot_to_protocol(&event.state).map_err(anyhow::Error::msg)?,
    )))?;
    journal.publish(DaemonEvent::JobChanged(JobSummary {
        id: event.job_id,
        kind: JobKind::Raw,
        label: format!("Raw {command}"),
        lifecycle: event.lifecycle,
        progress_current: None,
        progress_total: None,
        exit_code: event.exit_code,
    }))?;
    Ok(())
}
