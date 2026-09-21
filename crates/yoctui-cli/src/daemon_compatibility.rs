use std::collections::BTreeMap;
use std::time::Duration;

use thiserror::Error;
use yoctui_bitbake::{
    CapabilityCacheError, CapabilityProbeContext, CapabilityProbeRunner, CapabilityResolver,
    CapabilitySnapshotCache,
};
use yoctui_model::{
    CapabilityCacheKey, CapabilityCatalog, CapabilityCatalogError, CapabilityId,
    CapabilityImplementation, DaemonCompatibilitySnapshot,
};

mod coordinator;
mod process_helpers;
mod runtime_detection;

pub use coordinator::spawn_startup;

#[cfg(test)]
use coordinator::probe_tool;
#[cfg(test)]
use process_helpers::{authoritative_token, authoritative_value, discover_executable};

const STARTUP_QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_QUERY_OUTPUT_LIMIT: usize = 64 * 1024;
const STARTUP_FINGERPRINT_LIMIT: usize = 4 * 1024 * 1024;
const STARTUP_ENVIRONMENT_LIMIT: usize = 1_024;
const STARTUP_ENVIRONMENT_VALUE_LIMIT: usize = 4_096;
const PROBE_CONCURRENCY: usize = 8;

#[derive(Debug, Clone)]
pub struct DaemonCompatibilityRuntime {
    pub key: CapabilityCacheKey,
    pub context: CapabilityProbeContext,
}

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
    #[error(transparent)]
    ProbeContext(#[from] yoctui_bitbake::CapabilityProbeContextError),
    #[error(transparent)]
    Identity(#[from] yoctui_model::EnvironmentIdentityError),
    #[error("invalid initialized daemon environment: {0}")]
    InvalidStartupEnvironment(String),
    #[error("daemon startup capability query failed: {0}")]
    StartupProbe(String),
    #[error("capability probe context belongs to another environment")]
    EnvironmentMismatch,
    #[error("capability probe result is stale for the selected daemon environment")]
    StaleProbe,
}

#[cfg(test)]
#[path = "tests/daemon_compatibility/mod.rs"]
mod tests;
