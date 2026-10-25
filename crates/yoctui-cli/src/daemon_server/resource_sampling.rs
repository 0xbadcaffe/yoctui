use super::*;
use crate::build_resource_sampler::{BuildResourceSampler, TaskRoot};

#[derive(Default)]
pub(super) struct ResourceSampling {
    sampler: BuildResourceSampler,
    pending: Option<
        tokio::task::JoinHandle<(BuildResourceSampler, yoctui_model::BuildResourceTelemetry)>,
    >,
    last_started_ms: u64,
    pub(super) latest: Option<yoctui_model::BuildResourceTelemetry>,
}
impl ResourceSampling {
    pub(super) async fn poll(&mut self, services: &DaemonServices, clients: usize, now: u64) {
        if self.pending.as_ref().is_some_and(|h| h.is_finished())
            && let Some(handle) = self.pending.take()
            && let Ok((sampler, sample)) = handle.await
        {
            self.sampler = sampler;
            self.latest = sample.valid().then_some(sample);
        }
        let snapshot = services.daemon_journal.snapshot();
        let job = snapshot
            .jobs
            .iter()
            .rev()
            .find(|j| j.kind == yoctui_protocol::daemon::JobKind::BitBakeBuild);
        let active = job.is_some_and(|j| {
            matches!(
                j.lifecycle,
                yoctui_protocol::daemon::LifecycleState::Connecting
                    | yoctui_protocol::daemon::LifecycleState::Running
                    | yoctui_protocol::daemon::LifecycleState::Stopping
            )
        });
        if self.pending.is_some()
            || (!active && clients == 0)
            || now.saturating_sub(self.last_started_ms) < if active { 1000 } else { 5000 }
        {
            return;
        }
        let build_dir = snapshot
            .workspace
            .as_ref()
            .map(|w| PathBuf::from(&w.canonical_build))
            .or_else(|| {
                snapshot
                    .compatibility
                    .as_ref()
                    .and_then(|c| match &c.environment.build_directory {
                        yoctui_protocol::daemon::CompatibilityDetected::Detected {
                            value, ..
                        } => Some(PathBuf::from(value)),
                        yoctui_protocol::daemon::CompatibilityDetected::Unknown => None,
                    })
            });
        let Some(build_dir) = build_dir else {
            return;
        };
        let mut roots = BTreeMap::new();
        for event in &snapshot.build_events {
            use yoctui_protocol::daemon::DaemonBuildEvent as E;
            match event {
                E::Reset { .. } => roots.clear(),
                E::TaskStarted {
                    recipe,
                    task,
                    pid: Some(pid),
                    ..
                } => {
                    roots.insert((recipe.clone(), task.clone()), *pid);
                }
                E::TaskCompleted { recipe, task, .. } => {
                    roots.remove(&(recipe.clone(), task.clone()));
                }
                _ => {}
            }
        }
        let roots = if active {
            roots
                .into_iter()
                .take(129)
                .map(|((recipe, task), pid)| TaskRoot { recipe, task, pid })
                .collect()
        } else {
            vec![]
        };
        let build_id = job.map(|j| j.id.0);
        let profile = services.bitbake_supervisor.performance_profile();
        let mut sampler = std::mem::take(&mut self.sampler);
        self.pending = Some(tokio::task::spawn_blocking(move || {
            let sample = sampler.sample(&build_dir, &roots, build_id, profile);
            (sampler, sample)
        }));
        self.last_started_ms = now;
    }
}
