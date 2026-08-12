use crate::{BackgroundJobId, BackgroundJobStatus, BuildStatus, DaemonJobState};

pub const MAX_BITBAKE_RESTART_AFFECTED_JOBS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeRestartJobId {
    PrimaryBuild,
    Background(BackgroundJobId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeRestartAffectedJob {
    pub id: BitBakeRestartJobId,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeRestartPreview {
    pub controller_generation: u64,
    pub server_identity: String,
    pub affected_jobs: Vec<BitBakeRestartAffectedJob>,
}

impl BitBakeRestartPreview {
    pub fn confirmation(&self) -> BitBakeRestartConfirmation {
        BitBakeRestartConfirmation {
            controller_generation: self.controller_generation,
            server_identity: self.server_identity.clone(),
            affected_job_ids: self
                .affected_jobs
                .iter()
                .map(|job| job.id.clone())
                .collect(),
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        !self.affected_jobs.is_empty()
    }

    pub fn validate_confirmation(&self, confirmation: &BitBakeRestartConfirmation) -> bool {
        &self.confirmation() == confirmation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeRestartConfirmation {
    pub controller_generation: u64,
    pub server_identity: String,
    pub affected_job_ids: Vec<BitBakeRestartJobId>,
}

pub fn bitbake_restart_affected_jobs(jobs: &DaemonJobState) -> Vec<BitBakeRestartAffectedJob> {
    let mut affected = Vec::new();
    if matches!(
        jobs.build.status,
        BuildStatus::LoadingWorkspace
            | BuildStatus::Parsing
            | BuildStatus::Running
            | BuildStatus::Cancelling
    ) {
        affected.push(BitBakeRestartAffectedJob {
            id: BitBakeRestartJobId::PrimaryBuild,
            title: jobs.build.target.as_deref().map_or_else(
                || "BitBake build".into(),
                |target| format!("Build {target}"),
            ),
            status: format!("{:?}", jobs.build.status),
        });
    }
    affected.extend(
        jobs.background_jobs
            .jobs
            .iter()
            .filter(|job| !job.status.is_terminal())
            .take(MAX_BITBAKE_RESTART_AFFECTED_JOBS.saturating_sub(affected.len()))
            .map(|job| BitBakeRestartAffectedJob {
                id: BitBakeRestartJobId::Background(job.id),
                title: job.title.clone(),
                status: background_status(job.status).into(),
            }),
    );
    affected
}

fn background_status(status: BackgroundJobStatus) -> &'static str {
    match status {
        BackgroundJobStatus::Queued => "Queued",
        BackgroundJobStatus::Starting => "Starting",
        BackgroundJobStatus::Running => "Running",
        BackgroundJobStatus::Cancelling => "Cancelling",
        BackgroundJobStatus::Succeeded => "Succeeded",
        BackgroundJobStatus::Failed => "Failed",
        BackgroundJobStatus::Cancelled => "Cancelled",
        BackgroundJobStatus::Lost => "Lost",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, BackgroundJobContext, BackgroundJobKind, BackgroundJobSpec, DaemonJobState};
    use std::time::SystemTime;

    #[test]
    fn bitbake_restart_lists_active_work_and_requires_exact_confirmation() {
        let mut app = App::new(128, 1024 * 1024);
        app.build.status = BuildStatus::Running;
        app.build.target = Some("core-image-minimal".into());
        crate::update(
            &mut app,
            crate::Action::QueueBackgroundJob(BackgroundJobSpec {
                id: BackgroundJobId(9),
                kind: BackgroundJobKind::Sdk,
                title: "SDK build".into(),
                context: BackgroundJobContext::default(),
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let jobs = DaemonJobState::capture(&app);
        let affected = bitbake_restart_affected_jobs(&jobs);
        assert_eq!(affected.len(), 2);
        let preview = BitBakeRestartPreview {
            controller_generation: 4,
            server_identity: "server-a".into(),
            affected_jobs: affected,
        };
        assert!(preview.requires_confirmation());
        assert!(preview.validate_confirmation(&preview.confirmation()));
        let mut stale = preview.confirmation();
        stale.controller_generation += 1;
        assert!(!preview.validate_confirmation(&stale));
    }

    #[test]
    fn bitbake_restart_ignores_terminal_work() {
        let jobs = DaemonJobState::capture(&App::new(128, 1024 * 1024));
        assert!(bitbake_restart_affected_jobs(&jobs).is_empty());
        let preview = BitBakeRestartPreview {
            controller_generation: 1,
            server_identity: "server".into(),
            affected_jobs: Vec::new(),
        };
        assert!(!preview.requires_confirmation());
    }
}
