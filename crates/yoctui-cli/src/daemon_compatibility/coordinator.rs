use std::{collections::BTreeMap, sync::Arc, time::Duration};

use yoctui_bitbake::{
    CapabilityProbeContext, CapabilityProbeObservation, CapabilityProbeRunner, CapabilityResolver,
    CapabilitySnapshotCache,
};
use yoctui_model::{
    CapabilityCacheKey, CapabilityCatalog, CapabilityId, CapabilityImplementation,
    DaemonCompatibilitySnapshot,
};

use super::{
    DaemonCompatibilityCoordinator, DaemonCompatibilityError, DaemonCompatibilityProbeTicket,
    DaemonCompatibilityRuntime, DaemonCompatibilitySelection, PROBE_CONCURRENCY,
};

pub fn spawn_startup(
    environment: BTreeMap<String, String>,
) -> crate::daemon_metadata::StartupMetadata<DaemonCompatibilitySnapshot> {
    crate::daemon_metadata::StartupMetadata::spawn(|mut cancelled| async move {
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        tokio::select! {
            biased;
            _ = &mut cancelled => anyhow::bail!("startup compatibility discovery cancelled"),
            result = tokio::time::timeout(Duration::from_secs(600), coordinator.startup_from_environment(&environment)) => {
                result.map_err(|_| anyhow::anyhow!("startup compatibility discovery exceeded ten minutes"))?
                    .map_err(anyhow::Error::from)
            }
        }
    })
}

impl Default for DaemonCompatibilityCoordinator {
    fn default() -> Self {
        Self {
            cache: CapabilitySnapshotCache::default(),
            catalog: CapabilityCatalog::builtin(),
            resolver: CapabilityResolver::default(),
            runner: CapabilityProbeRunner::default().with_background_priority(),
            active_key: None,
            implementations: BTreeMap::new(),
        }
    }
}

impl DaemonCompatibilityCoordinator {
    pub async fn startup_from_environment(
        &mut self,
        environment: &BTreeMap<String, String>,
    ) -> Result<Option<DaemonCompatibilitySnapshot>, DaemonCompatibilityError> {
        let Some(runtime) = DaemonCompatibilityRuntime::detect(environment).await? else {
            return Ok(None);
        };
        match self.select_environment(runtime.key)? {
            DaemonCompatibilitySelection::Cached(snapshot) => Ok(Some(snapshot)),
            DaemonCompatibilitySelection::Probe(ticket) => {
                self.probe(ticket, &runtime.context).await.map(Some)
            }
        }
    }

    pub fn select_environment(
        &mut self,
        key: CapabilityCacheKey,
    ) -> Result<DaemonCompatibilitySelection, DaemonCompatibilityError> {
        self.catalog.validate()?;
        let selection = self.cache.select(key.clone())?;
        if self.active_key.as_ref() != Some(&key) {
            self.active_key = Some(key.clone());
            self.implementations.clear();
        }
        if let Some(snapshot) = selection.snapshot {
            let compatibility = DaemonCompatibilitySnapshot {
                snapshot,
                implementations: self.implementations.clone(),
            }
            .normalize()?;
            return Ok(DaemonCompatibilitySelection::Cached(compatibility));
        }
        Ok(DaemonCompatibilitySelection::Probe(
            DaemonCompatibilityProbeTicket {
                key,
                generation: selection.generation,
            },
        ))
    }

    pub async fn probe(
        &mut self,
        ticket: DaemonCompatibilityProbeTicket,
        context: &CapabilityProbeContext,
    ) -> Result<DaemonCompatibilitySnapshot, DaemonCompatibilityError> {
        if !context.matches_environment(&ticket.key.environment) {
            return Err(DaemonCompatibilityError::EnvironmentMismatch);
        }
        let mut unique_probes = Vec::<yoctui_model::CapabilityProbeSpec>::new();
        for entry in &self.catalog.entries {
            for probe in &entry.probes {
                if !unique_probes.contains(probe) {
                    unique_probes.push(probe.clone());
                }
            }
        }
        let semaphore = Arc::new(tokio::sync::Semaphore::new(PROBE_CONCURRENCY));
        let tool_semaphores = unique_probes
            .iter()
            .filter_map(probe_tool)
            .map(|tool| (tool, Arc::new(tokio::sync::Semaphore::new(1))))
            .collect::<BTreeMap<_, _>>();
        let mut tasks = tokio::task::JoinSet::new();
        for (index, probe) in unique_probes.iter().cloned().enumerate() {
            let semaphore = Arc::clone(&semaphore);
            let tool_semaphore = probe_tool(&probe)
                .and_then(|tool| tool_semaphores.get(&tool))
                .cloned();
            let runner = self.runner.clone();
            let context = context.clone();
            tasks.spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))?;
                // Help/version commands for one executable can share caches,
                // locks, or workspace initialization. Running them in
                // parallel makes individually bounded probes time each other
                // out (notably Devtool on supported Poky). Keep cross-tool
                // concurrency while serializing commands for the same tool.
                let _tool_permit = match tool_semaphore {
                    Some(semaphore) => Some(semaphore.acquire_owned().await.map_err(|error| {
                        DaemonCompatibilityError::StartupProbe(error.to_string())
                    })?),
                    None => None,
                };
                Ok::<_, DaemonCompatibilityError>((index, runner.probe(&context, &probe).await))
            });
        }
        let mut completed = vec![None; unique_probes.len()];
        while let Some(result) = tasks.join_next().await {
            let (index, observation) = result
                .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))??;
            completed[index] = Some(observation);
        }
        let mut observations = BTreeMap::<CapabilityId, Vec<CapabilityProbeObservation>>::new();
        for entry in &self.catalog.entries {
            observations.insert(
                entry.id,
                entry
                    .probes
                    .iter()
                    .map(|probe| {
                        let index = unique_probes
                            .iter()
                            .position(|candidate| candidate == probe)
                            .expect("catalog probe was indexed before execution");
                        completed[index]
                            .clone()
                            .expect("every indexed probe task must complete")
                    })
                    .collect(),
            );
        }
        let resolved = self.resolver.resolve_snapshot(
            ticket.generation,
            ticket.key.environment.clone(),
            &self.catalog,
            &observations,
        )?;
        self.accept(ticket, resolved.snapshot, resolved.implementations)
    }

    pub fn accept(
        &mut self,
        ticket: DaemonCompatibilityProbeTicket,
        snapshot: yoctui_model::CapabilitySnapshot,
        implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
    ) -> Result<DaemonCompatibilitySnapshot, DaemonCompatibilityError> {
        if self.active_key.as_ref() != Some(&ticket.key)
            || self.cache.generation() != ticket.generation
        {
            return Err(DaemonCompatibilityError::StaleProbe);
        }
        let compatibility = DaemonCompatibilitySnapshot {
            snapshot,
            implementations,
        }
        .normalize()?;
        self.cache.store(
            &ticket.key,
            ticket.generation,
            compatibility.snapshot.clone(),
        )?;
        self.implementations = compatibility.implementations.clone();
        Ok(compatibility)
    }

    pub fn invalidate(&mut self) -> Result<u64, DaemonCompatibilityError> {
        self.active_key = None;
        self.implementations.clear();
        Ok(self.cache.invalidate()?)
    }
}

pub(super) fn probe_tool(
    probe: &yoctui_model::CapabilityProbeSpec,
) -> Option<yoctui_model::CapabilityToolId> {
    use yoctui_model::CapabilityProbeSpec;
    match probe {
        CapabilityProbeSpec::Executable { tool }
        | CapabilityProbeSpec::CommandVersion { tool }
        | CapabilityProbeSpec::CommandHelp { tool, .. }
        | CapabilityProbeSpec::CommandOption { tool, .. }
        | CapabilityProbeSpec::CommandHelpText { tool, .. } => Some(*tool),
        CapabilityProbeSpec::MetadataAnyTask { .. }
        | CapabilityProbeSpec::MetadataVariable { .. }
        | CapabilityProbeSpec::BackendCapability { .. }
        | CapabilityProbeSpec::ProtocolCapability { .. }
        | CapabilityProbeSpec::Artifact { .. }
        | CapabilityProbeSpec::Configuration { .. } => None,
    }
}
