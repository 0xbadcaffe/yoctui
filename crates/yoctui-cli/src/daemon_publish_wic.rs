//! Daemon publish wic.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_wic_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_wic::DaemonWicEvent,
) -> Result<()> {
    use daemon_wic::DaemonWicEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = match &event {
        DaemonWicEvent::Started { job_id, .. }
        | DaemonWicEvent::Output { job_id, .. }
        | DaemonWicEvent::Completed { job_id, .. }
        | DaemonWicEvent::Failed { job_id, .. }
        | DaemonWicEvent::Cancelled { job_id, .. }
        | DaemonWicEvent::Lost { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "Wic".into());
    let mapped = match event {
        DaemonWicEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Wic,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonWicEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("wic:{stream:?}"),
            severity: if matches!(stream, yoctui_bitbake::WicRunnerOutputStream::Stderr) {
                LogSeverity::Warning
            } else {
                LogSeverity::Info
            },
            message: if truncated {
                format!("{line} [truncated]")
            } else {
                line
            },
            unix_ms: unix_ms(),
            recipe: None,
            task: None,
            path: None,
            build: None,
        }),
        DaemonWicEvent::Completed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Wic,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(exit_code),
        }),
        DaemonWicEvent::Failed {
            exit_code, message, ..
        } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Wic,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonWicEvent::Cancelled { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Wic,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonWicEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Wic,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Lost,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
    };
    journal.publish(mapped)?;
    Ok(())
}
