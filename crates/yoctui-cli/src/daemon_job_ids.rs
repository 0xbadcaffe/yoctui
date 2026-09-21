use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use yoctui_protocol::daemon::{DaemonSnapshot, JobId};

// Raw jobs and PTYs occupy separate high-bit namespaces. Ordinary daemon jobs
// share this low namespace regardless of the supervisor that owns their work.
const JOB_ID_LIMIT: u64 = 1 << 60;

#[derive(Clone, Debug)]
pub struct DaemonJobIds(Arc<AtomicU64>);

impl Default for DaemonJobIds {
    fn default() -> Self {
        Self(Arc::new(AtomicU64::new(1)))
    }
}

impl DaemonJobIds {
    pub fn from_snapshot(snapshot: &DaemonSnapshot) -> Self {
        let next = snapshot
            .jobs
            .iter()
            .map(|job| job.id.0)
            .filter(|id| *id < JOB_ID_LIMIT)
            .max()
            .unwrap_or(0)
            + 1;
        Self(Arc::new(AtomicU64::new(next)))
    }

    pub fn allocate(&self) -> Result<JobId, &'static str> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                (next < JOB_ID_LIMIT).then(|| next + 1)
            })
            .map(JobId)
            .map_err(|_| "daemon job ID space exhausted")
    }
}

#[cfg(test)]
#[path = "tests/daemon_job_ids/mod.rs"]
mod tests;
