//! Daemon publish devtool.
use super::*;

pub(crate) fn publish_daemon_devtool_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_devtool::DaemonDevtoolEvent,
) -> Result<()> {
    use daemon_devtool::DaemonDevtoolEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = event.job_id();
    let existing = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .cloned();
    let label = existing
        .as_ref()
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "Devtool".into());
    let mapped = match event {
        DaemonDevtoolEvent::Started { label, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Devtool,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonDevtoolEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("devtool:{stream:?}"),
            severity: if matches!(stream, yoctui_bitbake::DevtoolOutputStream::Stderr) {
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
        DaemonDevtoolEvent::Completed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Devtool,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonDevtoolEvent::Failed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Devtool,
            label,
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonDevtoolEvent::Cancelled { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Devtool,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonDevtoolEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Devtool,
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
