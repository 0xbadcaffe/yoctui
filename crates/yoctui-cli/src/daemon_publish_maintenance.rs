//! Daemon publish maintenance.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_maintenance_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_maintenance::DaemonMaintenanceEvent,
) -> Result<()> {
    use daemon_maintenance::DaemonMaintenanceEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = match &event {
        DaemonMaintenanceEvent::Started { job_id, .. }
        | DaemonMaintenanceEvent::Output { job_id, .. }
        | DaemonMaintenanceEvent::Completed { job_id, .. }
        | DaemonMaintenanceEvent::Failed { job_id, .. }
        | DaemonMaintenanceEvent::Cancelled { job_id, .. }
        | DaemonMaintenanceEvent::Lost { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "Maintenance".into());
    let mapped = match event {
        DaemonMaintenanceEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Maintenance,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonMaintenanceEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("maintenance:{stream:?}"),
            severity: if matches!(stream, yoctui_model::MaintenanceOutputStream::Stderr) {
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
        DaemonMaintenanceEvent::Completed { exit_code, .. } => {
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Maintenance,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code,
            })
        }
        DaemonMaintenanceEvent::Failed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Maintenance,
            label,
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonMaintenanceEvent::Cancelled { exit_code, .. } => {
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Maintenance,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code,
            })
        }
        DaemonMaintenanceEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Maintenance,
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
