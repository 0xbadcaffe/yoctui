use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

use yoctui_bitbake::{CapabilityProbeObservation, CapabilityProbeStatus, CapabilityResolver};
use yoctui_model::{
    CapabilityCatalog, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityProbeSpec, CapabilityState, DaemonCompatibilitySnapshot,
};

use crate::daemon_metadata::StartupMetadata;

const RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(crate) struct BackendRecovery {
    worker: Option<StartupMetadata<DaemonCompatibilitySnapshot>>,
    next_attempt: Option<Instant>,
}

impl BackendRecovery {
    pub(crate) fn poll(
        &mut self,
        current: Option<&DaemonCompatibilitySnapshot>,
        environment: &BTreeMap<String, String>,
    ) -> Option<anyhow::Result<Option<DaemonCompatibilitySnapshot>>> {
        if let Some(result) = self.worker.as_mut().and_then(StartupMetadata::try_result) {
            self.worker = None;
            self.next_attempt = Some(Instant::now() + RETRY_DELAY);
            return Some(result);
        }
        let current = current?;
        if self.worker.is_some()
            || self
                .next_attempt
                .is_some_and(|deadline| Instant::now() < deadline)
            || environment.contains_key("YOCTUI_BRIDGE_PATH")
            || !needed(current)
        {
            return None;
        }
        let current = current.clone();
        let environment = environment.clone();
        self.worker = Some(StartupMetadata::spawn(|mut cancelled| async move {
            tokio::select! {
                biased;
                _ = &mut cancelled => anyhow::bail!("BitBake API discovery cancelled"),
                result = recover(current, &environment) => result.map(Some),
            }
        }));
        None
    }

    pub(crate) async fn shutdown(&mut self) {
        if let Some(worker) = self.worker.as_mut() {
            worker.shutdown().await;
        }
    }
}

pub(crate) fn needed(current: &DaemonCompatibilitySnapshot) -> bool {
    CapabilityCatalog::builtin().entries.iter().any(|entry| {
        backend_only(&entry.probes)
            && current
                .snapshot
                .capability(entry.id)
                .is_some_and(|record| matches!(record.state, CapabilityState::Unknown { .. }))
    })
}

fn backend_only(probes: &[CapabilityProbeSpec]) -> bool {
    !probes.is_empty()
        && probes
            .iter()
            .all(|probe| matches!(probe, CapabilityProbeSpec::BackendCapability { .. }))
}

pub(crate) fn is_current_recovery(
    current: &DaemonCompatibilitySnapshot,
    recovered: &DaemonCompatibilitySnapshot,
) -> bool {
    current.snapshot.generation.checked_add(1) == Some(recovered.snapshot.generation)
        && current.snapshot.environment == recovered.snapshot.environment
}

pub(crate) async fn recover(
    mut current: DaemonCompatibilitySnapshot,
    environment: &BTreeMap<String, String>,
) -> anyhow::Result<DaemonCompatibilitySnapshot> {
    let identity = &current.snapshot.environment;
    let build = identity
        .build_directory
        .value()
        .ok_or_else(|| anyhow::anyhow!("No verified build directory for BitBake API discovery"))?;
    let version = identity
        .bitbake_version
        .value()
        .ok_or_else(|| anyhow::anyhow!("No verified BitBake version for API discovery"))?;
    let configured = environment
        .get("BUILDDIR")
        .ok_or_else(|| anyhow::anyhow!("Missing initialized BUILDDIR"))?;
    anyhow::ensure!(
        Path::new(configured).canonicalize()? == *build,
        "BitBake API recovery environment changed"
    );
    let python = environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let capabilities = yoctui_bitbake::probe_bundled_backend_capabilities(
        Path::new(python),
        build,
        &super::process_helpers::bounded_process_environment(environment),
        version,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    let resolver = CapabilityResolver::default();
    for entry in CapabilityCatalog::builtin().entries {
        if !backend_only(&entry.probes) {
            continue;
        }
        let observations = entry
            .probes
            .iter()
            .map(|probe| {
                let CapabilityProbeSpec::BackendCapability { name } = probe else {
                    unreachable!()
                };
                let present = capabilities.contains(name);
                CapabilityProbeObservation {
                    status: if present {
                        CapabilityProbeStatus::Positive
                    } else {
                        CapabilityProbeStatus::Negative
                    },
                    evidence: CapabilityEvidence {
                        kind: CapabilityEvidenceKind::BackendNegotiation,
                        outcome: if present {
                            CapabilityEvidenceOutcome::Positive
                        } else {
                            CapabilityEvidenceOutcome::Negative
                        },
                        subject: "BitBake backend capability".into(),
                        detail: format!(
                            "Current build API probe reports {name}: {}",
                            if present { "available" } else { "unavailable" }
                        ),
                        argv: Vec::new(),
                    },
                }
            })
            .collect::<Vec<_>>();
        let resolved = resolver.resolve(&entry, Some(version), &observations);
        // A negative API report does not invalidate independently probed command fallbacks.
        let keep_command_fallback = resolved.implementation.is_none()
            && current
                .implementations
                .get(&entry.id)
                .is_some_and(|implementation| {
                    implementation.kind == yoctui_model::CapabilityImplementationKind::Command
                        && entry
                            .fallback
                            .as_ref()
                            .is_some_and(|fallback| fallback.implementation == *implementation)
                });
        if keep_command_fallback {
            continue;
        }
        if let Some(record) = current
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == entry.id)
        {
            *record = resolved.record;
        }
        if let Some(implementation) = resolved.implementation {
            current.implementations.insert(entry.id, implementation);
        } else {
            current.implementations.remove(&entry.id);
        }
    }
    current.snapshot.generation = current
        .snapshot
        .generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Capability generation exhausted"))?;
    Ok(current.normalize()?)
}
