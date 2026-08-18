use sha2::{Digest, Sha256};
use thiserror::Error;
use yoctui_model::{
    CapabilityCacheKey, CapabilityCacheKeyError, CapabilitySnapshot, YoctoEnvironmentIdentity,
};

const MAX_FINGERPRINT_MATERIAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CapabilityFingerprintMaterial<'a> {
    pub workspace_identity: &'a str,
    pub initialized_environment: &'a [u8],
    pub layer_configuration: &'a [u8],
    pub build_configuration: &'a [u8],
    pub daemon_workspace_identity: &'a str,
}

impl CapabilityFingerprintMaterial<'_> {
    pub fn key(
        &self,
        environment: YoctoEnvironmentIdentity,
    ) -> Result<CapabilityCacheKey, CapabilityCacheError> {
        if [
            self.initialized_environment.len(),
            self.layer_configuration.len(),
            self.build_configuration.len(),
        ]
        .into_iter()
        .any(|size| size > MAX_FINGERPRINT_MATERIAL_BYTES)
        {
            return Err(CapabilityCacheError::OversizedFingerprintMaterial);
        }
        CapabilityCacheKey {
            environment,
            workspace_identity: self.workspace_identity.into(),
            initialized_environment_digest: digest(self.initialized_environment),
            layer_configuration_digest: digest(self.layer_configuration),
            build_configuration_digest: digest(self.build_configuration),
            daemon_workspace_identity: self.daemon_workspace_identity.into(),
        }
        .normalize()
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCacheSelection {
    pub generation: u64,
    pub snapshot: Option<CapabilitySnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct CapabilitySnapshotCache {
    generation: u64,
    active_key: Option<CapabilityCacheKey>,
    snapshot: Option<CapabilitySnapshot>,
}

impl CapabilitySnapshotCache {
    pub fn select(
        &mut self,
        key: CapabilityCacheKey,
    ) -> Result<CapabilityCacheSelection, CapabilityCacheError> {
        let key = key.normalize()?;
        if self.active_key.as_ref() != Some(&key) {
            self.advance_generation()?;
            self.active_key = Some(key);
            self.snapshot = None;
        } else if self.generation == 0 {
            return Err(CapabilityCacheError::InvalidState);
        }
        Ok(CapabilityCacheSelection {
            generation: self.generation,
            snapshot: self.snapshot.clone(),
        })
    }

    pub fn store(
        &mut self,
        key: &CapabilityCacheKey,
        generation: u64,
        snapshot: CapabilitySnapshot,
    ) -> Result<(), CapabilityCacheError> {
        let key = key.clone().normalize()?;
        if self.active_key.as_ref() != Some(&key)
            || generation == 0
            || generation != self.generation
            || snapshot.generation != generation
            || snapshot.environment != key.environment
        {
            return Err(CapabilityCacheError::StaleOrMismatched);
        }
        self.snapshot = Some(snapshot.normalize()?);
        Ok(())
    }

    pub fn lookup(&self, key: &CapabilityCacheKey) -> Option<&CapabilitySnapshot> {
        (self.active_key.as_ref() == Some(key))
            .then_some(self.snapshot.as_ref())
            .flatten()
    }

    pub fn invalidate(&mut self) -> Result<u64, CapabilityCacheError> {
        if self.active_key.is_none() {
            return Ok(self.generation);
        }
        self.advance_generation()?;
        self.snapshot = None;
        Ok(self.generation)
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    fn advance_generation(&mut self) -> Result<(), CapabilityCacheError> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(CapabilityCacheError::GenerationExhausted)?;
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityCacheError {
    #[error(transparent)]
    InvalidKey(#[from] CapabilityCacheKeyError),
    #[error(transparent)]
    InvalidSnapshot(#[from] yoctui_model::CapabilityModelError),
    #[error("capability fingerprint material exceeds its safety bound")]
    OversizedFingerprintMaterial,
    #[error("capability cache generation is exhausted")]
    GenerationExhausted,
    #[error("capability cache is in an invalid state")]
    InvalidState,
    #[error("capability snapshot is stale or belongs to another environment")]
    StaleOrMismatched,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityRecord, CapabilityState, IdentityAuthority,
    };

    fn environment(build: &str, version: &str) -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                build.into(),
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                version.into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
    }

    fn material<'a>(workspace: &'a str, daemon: &'a str) -> CapabilityFingerprintMaterial<'a> {
        CapabilityFingerprintMaterial {
            workspace_identity: workspace,
            initialized_environment: b"PATH=/poky/bitbake/bin\0BUILDDIR=/poky/build",
            layer_configuration: b"BBLAYERS=/poky/meta /poky/meta-poky",
            build_configuration: b"MACHINE=qemux86-64\nDISTRO=poky",
            daemon_workspace_identity: daemon,
        }
    }

    fn snapshot(generation: u64, environment: YoctoEnvironmentIdentity) -> CapabilitySnapshot {
        CapabilitySnapshot {
            generation,
            environment,
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::BitBakeWorkspaceInspection,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::BackendNegotiation,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "workspace".into(),
                    detail: "backend reports workspace inspection".into(),
                    argv: Vec::new(),
                }],
            }],
        }
        .normalize()
        .unwrap()
    }

    #[test]
    fn compatibility_cache_reuses_only_exact_environment_key() {
        let env = environment("/poky/build", "2.8.1");
        let key = material("poky-one", "daemon-one").key(env.clone()).unwrap();
        let mut cache = CapabilitySnapshotCache::default();
        let first = cache.select(key.clone()).unwrap();
        assert_eq!(first.generation, 1);
        assert!(first.snapshot.is_none());
        cache
            .store(&key, first.generation, snapshot(first.generation, env))
            .unwrap();
        let reused = cache.select(key.clone()).unwrap();
        assert_eq!(reused.generation, 1);
        assert!(reused.snapshot.is_some());
        assert!(cache.lookup(&key).is_some());
    }

    #[test]
    fn compatibility_cache_invalidates_each_project_environment_and_config_dimension() {
        let base_environment = environment("/poky/build", "2.8.1");
        let base = material("poky-one", "daemon-one")
            .key(base_environment.clone())
            .unwrap();
        let mut changed_material = material("poky-one", "daemon-one");
        changed_material.layer_configuration = b"BBLAYERS=/poky/meta /external/meta-openembedded";
        let layer = changed_material.key(base_environment.clone()).unwrap();
        let changed_environment = environment("/poky/build", "2.10.0");
        let bitbake = material("poky-one", "daemon-one")
            .key(changed_environment)
            .unwrap();
        let project = material("poky-two", "daemon-two")
            .key(environment("/other/build", "2.8.1"))
            .unwrap();

        let mut cache = CapabilitySnapshotCache::default();
        for (expected, key) in [(1, base.clone()), (2, layer), (3, bitbake), (4, project)] {
            let selected = cache.select(key).unwrap();
            assert_eq!(selected.generation, expected);
            assert!(selected.snapshot.is_none());
            assert!(cache.lookup(&base).is_none());
        }
    }

    #[test]
    fn compatibility_cache_rejects_stale_generation_environment_and_key() {
        let base_environment = environment("/poky/build", "2.8.1");
        let key = material("poky-one", "daemon-one")
            .key(base_environment.clone())
            .unwrap();
        let other = material("poky-two", "daemon-two")
            .key(environment("/other/build", "2.8.1"))
            .unwrap();
        let mut cache = CapabilitySnapshotCache::default();
        let generation = cache.select(key.clone()).unwrap().generation;
        assert_eq!(
            cache.store(
                &key,
                generation + 1,
                snapshot(generation, base_environment.clone()),
            ),
            Err(CapabilityCacheError::StaleOrMismatched)
        );
        assert_eq!(
            cache.store(&other, generation, snapshot(generation, base_environment)),
            Err(CapabilityCacheError::StaleOrMismatched)
        );
        assert_eq!(cache.invalidate().unwrap(), 2);
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn compatibility_cache_fingerprint_is_deterministic_bounded_and_sensitive() {
        let base_environment = environment("/poky/build", "2.8.1");
        let first = material("poky-one", "daemon-one")
            .key(base_environment.clone())
            .unwrap();
        let second = material("poky-one", "daemon-one")
            .key(base_environment.clone())
            .unwrap();
        assert_eq!(first, second);
        let mut changed = material("poky-one", "daemon-one");
        changed.initialized_environment = b"PATH=/different";
        assert_ne!(first, changed.key(base_environment).unwrap());

        let oversized = vec![0; MAX_FINGERPRINT_MATERIAL_BYTES + 1];
        let invalid = CapabilityFingerprintMaterial {
            workspace_identity: "poky-one",
            initialized_environment: &oversized,
            layer_configuration: b"layers",
            build_configuration: b"build",
            daemon_workspace_identity: "daemon-one",
        };
        assert_eq!(
            invalid.key(environment("/poky/build", "2.8.1")),
            Err(CapabilityCacheError::OversizedFingerprintMaterial)
        );
    }

    #[test]
    fn compatibility_cache_generation_overflow_fails_closed() {
        let key = material("poky-one", "daemon-one")
            .key(environment("/poky/build", "2.8.1"))
            .unwrap();
        let mut cache = CapabilitySnapshotCache {
            generation: u64::MAX,
            active_key: Some(key),
            snapshot: None,
        };
        assert_eq!(
            cache.invalidate(),
            Err(CapabilityCacheError::GenerationExhausted)
        );
    }
}
