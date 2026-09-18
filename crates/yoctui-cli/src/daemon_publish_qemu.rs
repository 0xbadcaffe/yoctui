//! Daemon publish qemu.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_qemu_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_qemu::DaemonQemuEvent,
) -> Result<()> {
    use daemon_qemu::DaemonQemuEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = match &event {
        DaemonQemuEvent::Started { job_id, .. }
        | DaemonQemuEvent::Output { job_id, .. }
        | DaemonQemuEvent::Completed { job_id, .. }
        | DaemonQemuEvent::Failed { job_id, .. }
        | DaemonQemuEvent::Cancelled { job_id, .. }
        | DaemonQemuEvent::Lost { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "QEMU".into());
    let mapped = match event {
        DaemonQemuEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qemu,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonQemuEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("qemu:{stream:?}"),
            severity: if matches!(stream, yoctui_bitbake::QemuRunnerOutputStream::Stderr) {
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
        DaemonQemuEvent::Completed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qemu,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(exit_code),
        }),
        DaemonQemuEvent::Failed {
            exit_code, message, ..
        } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qemu,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonQemuEvent::Cancelled { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qemu,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonQemuEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qemu,
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
