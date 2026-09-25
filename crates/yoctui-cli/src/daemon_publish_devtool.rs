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
    if let DaemonDevtoolEvent::Status(status) = event {
        journal.publish(DaemonEvent::DevtoolStatusChanged(Box::new(
            yoctui_app::devtool_status_to_protocol(&status),
        )))?;
        return Ok(());
    }
    let job_id = event
        .job_id()
        .expect("non-status Devtool event has a job identity");
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
        DaemonDevtoolEvent::Status(_) => unreachable!("status returned before job mapping"),
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
