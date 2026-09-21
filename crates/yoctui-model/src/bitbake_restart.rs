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
#[path = "tests/bitbake_restart/mod.rs"]
mod tests;
