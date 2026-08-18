use std::collections::BTreeMap;

use thiserror::Error;
use yoctui_bitbake::{
    CapabilityCacheError, CapabilityProbeContext, CapabilityProbeObservation,
    CapabilityProbeRunner, CapabilityResolver, CapabilitySnapshotCache,
};
use yoctui_model::{
    CapabilityCacheKey, CapabilityCatalog, CapabilityCatalogError, CapabilityId,
    CapabilityImplementation, DaemonCompatibilitySnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCompatibilityProbeTicket {
    pub key: CapabilityCacheKey,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonCompatibilitySelection {
    Cached(DaemonCompatibilitySnapshot),
    Probe(DaemonCompatibilityProbeTicket),
}

/// Sole daemon owner of environment-correlated capability probing and cache
/// state. Clients receive its resolved snapshots through the daemon journal;
/// they never run this coordinator or infer release support themselves.
#[derive(Debug, Clone)]
pub struct DaemonCompatibilityCoordinator {
    cache: CapabilitySnapshotCache,
    catalog: CapabilityCatalog,
    resolver: CapabilityResolver,
    runner: CapabilityProbeRunner,
    active_key: Option<CapabilityCacheKey>,
    implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
}

impl Default for DaemonCompatibilityCoordinator {
    fn default() -> Self {
        Self {
            cache: CapabilitySnapshotCache::default(),
            catalog: CapabilityCatalog::builtin(),
            resolver: CapabilityResolver::default(),
            runner: CapabilityProbeRunner::default(),
            active_key: None,
            implementations: BTreeMap::new(),
        }
    }
}

impl DaemonCompatibilityCoordinator {
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
        let mut observations = BTreeMap::<CapabilityId, Vec<CapabilityProbeObservation>>::new();
        for entry in &self.catalog.entries {
            let mut entry_observations = Vec::with_capacity(entry.probes.len());
            for probe in &entry.probes {
                entry_observations.push(self.runner.probe(context, probe).await);
            }
            observations.insert(entry.id, entry_observations);
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

#[derive(Debug, Error)]
pub enum DaemonCompatibilityError {
    #[error(transparent)]
    Cache(#[from] CapabilityCacheError),
    #[error(transparent)]
    Catalog(#[from] CapabilityCatalogError),
    #[error(transparent)]
    Model(#[from] yoctui_model::CapabilityModelError),
    #[error(transparent)]
    State(#[from] yoctui_model::DaemonStateError),
    #[error("capability probe context belongs to another environment")]
    EnvironmentMismatch,
    #[error("capability probe result is stale for the selected daemon environment")]
    StaleProbe,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        path::PathBuf,
    };
    use yoctui_bitbake::CapabilityFingerprintMaterial;
    use yoctui_model::{AuthoritativeValue, IdentityAuthority, YoctoEnvironmentIdentity};

    fn environment(build: PathBuf, version: &str) -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                build,
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                version.into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
    }

    fn key(environment: YoctoEnvironmentIdentity, workspace: &str) -> CapabilityCacheKey {
        CapabilityFingerprintMaterial {
            workspace_identity: workspace,
            initialized_environment: b"PATH=/work/poky/bitbake/bin",
            layer_configuration: b"BBLAYERS=/work/poky/meta",
            build_configuration: b"MACHINE=qemux86-64\nDISTRO=poky",
            daemon_workspace_identity: workspace,
        }
        .key(environment)
        .unwrap()
    }

    fn context(environment: YoctoEnvironmentIdentity) -> CapabilityProbeContext {
        CapabilityProbeContext::new(
            environment.clone(),
            environment.build_directory.value().unwrap().clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::from(["do_build".into(), "do_populate_sdk".into()]),
            BTreeSet::from(["MACHINE".into(), "DISTRO".into()]),
            BTreeSet::from(["workspace_inspection".into()]),
            BTreeSet::from(["state_snapshots".into()]),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .unwrap()
    }

    fn temporary_build(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-daemon-compatibility-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }

    #[tokio::test]
    async fn daemon_compatibility_probes_once_and_reuses_one_snapshot_for_all_clients() {
        let build = temporary_build("reuse");
        let environment = environment(build.clone(), "2.18.0");
        let key = key(environment.clone(), "poky-current");
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let DaemonCompatibilitySelection::Probe(ticket) =
            coordinator.select_environment(key.clone()).unwrap()
        else {
            panic!("first environment selection must probe");
        };
        let resolved = coordinator
            .probe(ticket, &context(environment))
            .await
            .unwrap();
        assert_eq!(
            resolved.snapshot.capabilities.len(),
            CapabilityId::ALL.len()
        );

        let DaemonCompatibilitySelection::Cached(first_client) =
            coordinator.select_environment(key.clone()).unwrap()
        else {
            panic!("exact reconnect must reuse the daemon snapshot");
        };
        let DaemonCompatibilitySelection::Cached(second_client) =
            coordinator.select_environment(key).unwrap()
        else {
            panic!("second client must see the same daemon snapshot");
        };
        assert_eq!(first_client, second_client);
        assert_eq!(first_client, resolved);
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn daemon_compatibility_environment_change_rejects_stale_probe_result() {
        let first_build = temporary_build("first");
        let second_build = temporary_build("second");
        let first_environment = environment(first_build.clone(), "1.52.0");
        let second_environment = environment(second_build.clone(), "2.18.0");
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let DaemonCompatibilitySelection::Probe(stale) = coordinator
            .select_environment(key(first_environment, "poky-old"))
            .unwrap()
        else {
            panic!("first selection must probe");
        };
        let DaemonCompatibilitySelection::Probe(current) = coordinator
            .select_environment(key(second_environment.clone(), "poky-new"))
            .unwrap()
        else {
            panic!("changed environment must probe");
        };

        let current_snapshot = coordinator
            .probe(current, &context(second_environment))
            .await
            .unwrap();
        assert!(matches!(
            coordinator.accept(
                stale.clone(),
                current_snapshot.snapshot.clone(),
                current_snapshot.implementations.clone()
            ),
            Err(DaemonCompatibilityError::StaleProbe)
        ));
        assert!(matches!(
            coordinator.select_environment(stale.key).unwrap(),
            DaemonCompatibilitySelection::Probe(_)
        ));
        assert_eq!(coordinator.invalidate().unwrap(), 4);
        fs::remove_dir_all(first_build).unwrap();
        fs::remove_dir_all(second_build).unwrap();
    }
}
