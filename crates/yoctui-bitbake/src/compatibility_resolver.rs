use std::collections::BTreeMap;

use yoctui_model::{
    CapabilityCatalog, CapabilityCatalogEntry, CapabilityEvidence, CapabilityEvidenceKind,
    CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation, CapabilityReason,
    CapabilityRecord, CapabilitySnapshot, CapabilityState, FallbackSelector,
    YoctoEnvironmentIdentity,
};

use crate::{CapabilityProbeObservation, VersionFallbackMap, VersionFallbackResolution};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapability {
    pub record: CapabilityRecord,
    pub implementation: Option<CapabilityImplementation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapabilitySnapshot {
    pub snapshot: CapabilitySnapshot,
    pub implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityResolver {
    fallback: VersionFallbackMap,
}

impl CapabilityResolver {
    pub fn resolve(
        &self,
        entry: &CapabilityCatalogEntry,
        bitbake_version: Option<&str>,
        observations: &[CapabilityProbeObservation],
    ) -> ResolvedCapability {
        let evidence = observations
            .iter()
            .map(|observation| observation.evidence.clone())
            .collect::<Vec<_>>();
        match self
            .fallback
            .resolve_bitbake(entry, bitbake_version, observations)
        {
            VersionFallbackResolution::Inferred {
                implementation,
                state,
                evidence: fallback_evidence,
            } => ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state,
                    evidence: append_evidence(evidence, fallback_evidence),
                },
                implementation: Some(implementation),
            },
            VersionFallbackResolution::Unknown {
                state,
                evidence: fallback_evidence,
            } => ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state,
                    evidence: append_evidence(evidence, fallback_evidence),
                },
                implementation: None,
            },
            VersionFallbackResolution::Direct {
                outcome: CapabilityEvidenceOutcome::Positive,
            } => ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Available,
                    evidence,
                },
                implementation: Some(entry.preferred.clone()),
            },
            VersionFallbackResolution::Direct {
                outcome: CapabilityEvidenceOutcome::Negative,
            } => ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Unavailable {
                        reason: entry.unavailable_reason.clone(),
                    },
                    evidence,
                },
                implementation: None,
            },
            VersionFallbackResolution::Direct {
                outcome: CapabilityEvidenceOutcome::Inconclusive,
            } => ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Unknown {
                        reason: reason(
                            "evidence.resolution_mismatch",
                            "Direct evidence could not be resolved consistently.",
                            Some(entry.id.as_str()),
                        ),
                    },
                    evidence: append_evidence(
                        evidence,
                        CapabilityEvidence {
                            kind: yoctui_model::CapabilityEvidenceKind::ReleaseVersionFallback,
                            outcome: CapabilityEvidenceOutcome::Inconclusive,
                            subject: "direct evidence resolution".into(),
                            detail: "fallback resolver returned an inconclusive direct outcome"
                                .into(),
                            argv: Vec::new(),
                        },
                    ),
                },
                implementation: None,
            },
        }
    }

    pub fn resolve_snapshot(
        &self,
        generation: u64,
        environment: YoctoEnvironmentIdentity,
        catalog: &CapabilityCatalog,
        observations: &BTreeMap<CapabilityId, Vec<CapabilityProbeObservation>>,
    ) -> Result<ResolvedCapabilitySnapshot, yoctui_model::CapabilityModelError> {
        let bitbake_version = environment.bitbake_version.value().map(String::as_str);
        let mut records = Vec::with_capacity(catalog.entries.len());
        let mut implementations = BTreeMap::new();
        for entry in &catalog.entries {
            let resolved = self.resolve(
                entry,
                bitbake_version,
                observations
                    .get(&entry.id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            );
            if let Some(implementation) = resolved.implementation {
                implementations.insert(entry.id, implementation);
            }
            records.push(resolved.record);
        }
        for entry in &catalog.entries {
            if implementations.contains_key(&entry.id) {
                continue;
            }
            let Some(fallback) = entry.fallback.as_ref() else {
                continue;
            };
            let FallbackSelector::AvailableCapability { id: required } = &fallback.selector else {
                continue;
            };
            let required_available = records
                .iter()
                .find(|record| record.id == *required)
                .is_some_and(|record| record.state.is_enabled())
                && implementations.contains_key(required);
            if !required_available {
                continue;
            }
            let record = records
                .iter_mut()
                .find(|record| record.id == entry.id)
                .expect("catalog resolution must produce every capability record");
            record.state = CapabilityState::AvailableWithLimitations {
                reason: reason(
                    "fallback.available_capability",
                    "The preferred implementation is unavailable; a maintained capability-backed fallback was selected.",
                    Some(required.as_str()),
                ),
                limitations: vec![format!(
                    "Uses {} through implementation {}.",
                    required.as_str(),
                    fallback.implementation.id
                )],
            };
            record.evidence.push(CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Positive,
                subject: required.as_str().into(),
                detail: format!(
                    "Positive capability authority selected maintained fallback {}",
                    fallback.implementation.id
                ),
                argv: Vec::new(),
            });
            implementations.insert(entry.id, fallback.implementation.clone());
        }
        let snapshot = CapabilitySnapshot {
            generation,
            environment,
            capabilities: records,
        }
        .normalize()?;
        Ok(ResolvedCapabilitySnapshot {
            snapshot,
            implementations,
        })
    }
}

fn append_evidence(
    mut evidence: Vec<CapabilityEvidence>,
    fallback: CapabilityEvidence,
) -> Vec<CapabilityEvidence> {
    evidence.push(fallback);
    evidence
}

fn reason(code: &str, message: &str, capability: Option<&str>) -> CapabilityReason {
    CapabilityReason::new(
        code,
        message,
        capability.map(|id| format!("Required capability: {id}")),
    )
    .expect("static capability resolution reason must be valid")
}

#[cfg(test)]
#[path = "tests/compatibility_resolver/mod.rs"]
mod tests;
