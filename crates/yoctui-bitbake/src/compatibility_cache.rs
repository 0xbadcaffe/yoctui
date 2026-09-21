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
#[path = "tests/compatibility_cache/mod.rs"]
mod tests;
