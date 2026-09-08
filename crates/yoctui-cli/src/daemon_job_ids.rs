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
mod tests {
    use super::*;
    use yoctui_protocol::daemon::{JobKind, JobSummary, LifecycleState};

    fn snapshot(ids: &[u64]) -> DaemonSnapshot {
        let state = yoctui_model::DaemonGlobalState::new(
            yoctui_model::DaemonModelInstanceId([9; 16]),
            1,
            "fixture-boot".into(),
            yoctui_model::DaemonStateLimits::default(),
        )
        .unwrap();
        let mut snapshot = yoctui_app::daemon_protocol_snapshot(&state);
        snapshot.jobs = ids
            .iter()
            .map(|id| JobSummary {
                id: JobId(*id),
                kind: JobKind::BitBakeBuild,
                label: format!("recovered-{id}"),
                lifecycle: LifecycleState::Failed,
                progress_current: None,
                progress_total: None,
                exit_code: Some(1),
            })
            .collect();
        snapshot
    }

    #[test]
    fn daemon_job_identity_recovery_preserves_terminal_rows_and_raw_namespace() {
        use yoctui_protocol::daemon::{DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits};
        use yoctui_protocol::daemon_persist::{
            DaemonPersistedState, PersistedPreferences, recover_persisted_snapshot,
        };
        let mut before = snapshot(&[1, 2, (5 << 60) | 42]);
        before.jobs[2].kind = JobKind::Raw;
        let persisted = DaemonPersistedState::capture(
            &before,
            10,
            "fixture-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        let (recovered, _) = recover_persisted_snapshot(snapshot(&[]), &persisted, "fixture-boot");
        let ids = DaemonJobIds::from_snapshot(&recovered);
        let mut new_job = before.jobs[0].clone();
        new_job.id = ids.allocate().unwrap();
        new_job.label = "new build".into();
        new_job.lifecycle = LifecycleState::Connecting;
        new_job.exit_code = None;
        assert_eq!(new_job.id, JobId(3));
        let mut journal =
            DaemonSnapshotJournal::new(recovered, DaemonSnapshotLimits::default()).unwrap();
        journal.publish(DaemonEvent::JobChanged(new_job)).unwrap();
        assert_eq!(journal.snapshot().jobs.len(), 4);
        assert_eq!(&journal.snapshot().jobs[..3], &before.jobs);
        assert_eq!(
            DaemonJobIds::from_snapshot(journal.snapshot()).allocate(),
            Ok(JobId(4))
        );
    }

    #[test]
    fn daemon_job_identity_exhaustion_never_wraps_or_enters_reserved_namespace() {
        let ids = DaemonJobIds::from_snapshot(&snapshot(&[JOB_ID_LIMIT - 2]));
        assert_eq!(ids.allocate(), Ok(JobId(JOB_ID_LIMIT - 1)));
        for _ in 0..3 {
            assert_eq!(ids.allocate(), Err("daemon job ID space exhausted"));
        }
        let exhausted = DaemonJobIds::from_snapshot(&snapshot(&[JOB_ID_LIMIT - 1]));
        assert!(exhausted.allocate().is_err());
        assert_eq!(
            DaemonJobIds::from_snapshot(&snapshot(&[u64::MAX])).allocate(),
            Ok(JobId(1))
        );
    }

    #[test]
    fn daemon_job_identity_clones_allocate_unique_ids_concurrently() {
        let ids = DaemonJobIds::default();
        let threads = (0..8)
            .map(|_| {
                let ids = ids.clone();
                std::thread::spawn(move || {
                    (0..32)
                        .map(|_| ids.allocate().unwrap().0)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        let mut allocated = threads
            .into_iter()
            .flat_map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        allocated.sort_unstable();
        assert_eq!(allocated, (1..=256).collect::<Vec<_>>());
    }
}
