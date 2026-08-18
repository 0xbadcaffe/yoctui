use std::collections::BTreeMap;

use yoctui_model::{
    CapabilityCatalog, CapabilityCatalogEntry, CapabilityEvidence, CapabilityId,
    CapabilityImplementation, CapabilityReason, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, YoctoEnvironmentIdentity,
};

use crate::{
    CapabilityProbeObservation, CapabilityProbeStatus, VersionFallbackMap,
    VersionFallbackResolution,
};

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
        let positive = observations
            .iter()
            .any(|observation| observation.status == CapabilityProbeStatus::Positive);
        let negative = observations
            .iter()
            .any(|observation| observation.status == CapabilityProbeStatus::Negative);
        let evidence = observations
            .iter()
            .map(|observation| observation.evidence.clone())
            .collect::<Vec<_>>();
        if positive && negative {
            return ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Unknown {
                        reason: reason(
                            "evidence.conflict",
                            "Capability probes returned conflicting positive and negative evidence.",
                            Some(entry.id.as_str()),
                        ),
                    },
                    evidence,
                },
                implementation: None,
            };
        }
        if positive {
            return ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Available,
                    evidence,
                },
                implementation: Some(entry.preferred.clone()),
            };
        }
        if negative {
            return ResolvedCapability {
                record: CapabilityRecord {
                    id: entry.id,
                    state: CapabilityState::Unavailable {
                        reason: entry.unavailable_reason.clone(),
                    },
                    evidence,
                },
                implementation: None,
            };
        }

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
            VersionFallbackResolution::Direct { outcome } => {
                // The explicit positive/negative branches above consume all
                // direct conclusive evidence. Keep an impossible disagreement
                // fail-closed instead of manufacturing availability.
                ResolvedCapability {
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
                                outcome,
                                subject: "direct evidence resolution".into(),
                                detail: "fallback resolver returned direct evidence after conclusive evidence handling".into(),
                                argv: Vec::new(),
                            },
                        ),
                    },
                    implementation: None,
                }
            }
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
mod tests {
    use super::*;
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidenceKind, CapabilityEvidenceOutcome, IdentityAuthority,
        ReleaseIdentity,
    };

    fn observation(status: CapabilityProbeStatus, subject: &str) -> CapabilityProbeObservation {
        CapabilityProbeObservation {
            status,
            evidence: CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: match status {
                    CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                    CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                    CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
                },
                subject: subject.into(),
                detail: format!("synthetic future observation: {subject}"),
                argv: Vec::new(),
            },
        }
    }

    fn future_environment() -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                "/future/build".into(),
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                "99.0.0".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            oe_core: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("future-series".into()),
                    version: Some("99.0".into()),
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
    }

    #[test]
    fn compatibility_future_unknown_enables_only_positive_exact_capabilities() {
        let catalog = CapabilityCatalog::builtin();
        let observations = BTreeMap::from([
            (
                CapabilityId::DevtoolUpgrade,
                vec![observation(CapabilityProbeStatus::Positive, "upgrade")],
            ),
            (
                CapabilityId::ResultTool,
                vec![observation(CapabilityProbeStatus::Negative, "resulttool")],
            ),
            (
                CapabilityId::WicCreate,
                vec![
                    observation(CapabilityProbeStatus::Positive, "wic create"),
                    observation(CapabilityProbeStatus::Negative, "wic option"),
                ],
            ),
        ]);
        let resolved = CapabilityResolver::default()
            .resolve_snapshot(1, future_environment(), &catalog, &observations)
            .unwrap();
        assert!(resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
        assert!(!resolved.snapshot.allows(CapabilityId::ResultTool));
        assert!(!resolved.snapshot.allows(CapabilityId::WicCreate));
        assert!(!resolved.snapshot.allows(CapabilityId::RunQemu));
        assert!(matches!(
            resolved
                .snapshot
                .capability(CapabilityId::ResultTool)
                .unwrap()
                .state,
            CapabilityState::Unavailable { .. }
        ));
        assert!(matches!(
            resolved
                .snapshot
                .capability(CapabilityId::WicCreate)
                .unwrap()
                .state,
            CapabilityState::Unknown { .. }
        ));
        assert!(matches!(
            resolved
                .snapshot
                .capability(CapabilityId::RunQemu)
                .unwrap()
                .state,
            CapabilityState::Unknown { .. }
        ));
        assert_eq!(
            resolved.snapshot.capabilities.len(),
            CapabilityId::ALL.len()
        );
        assert_eq!(
            resolved
                .snapshot
                .environment
                .oe_core
                .value()
                .unwrap()
                .name
                .as_deref(),
            Some("future-series")
        );
    }

    #[test]
    fn compatibility_future_unknown_direct_probe_overrides_closed_static_boundary() {
        let catalog = CapabilityCatalog::builtin();
        let entry = catalog
            .entry(CapabilityId::BitBakeWorkspaceInspection)
            .unwrap();
        let resolved = CapabilityResolver::default().resolve(
            entry,
            Some("99.0"),
            &[observation(CapabilityProbeStatus::Positive, "workspace")],
        );
        assert_eq!(resolved.record.state, CapabilityState::Available);
        assert_eq!(resolved.implementation, Some(entry.preferred.clone()));
    }

    #[test]
    fn compatibility_future_unknown_absent_and_inconclusive_evidence_stays_unknown() {
        let catalog = CapabilityCatalog::builtin();
        let entry = catalog.entry(CapabilityId::BitBakeBuild).unwrap();
        for observations in [
            Vec::new(),
            vec![observation(CapabilityProbeStatus::Inconclusive, "build")],
        ] {
            let resolved = CapabilityResolver::default().resolve(entry, Some("3.0"), &observations);
            assert!(matches!(
                resolved.record.state,
                CapabilityState::Unknown { .. }
            ));
            assert!(resolved.implementation.is_none());
        }
    }

    #[test]
    fn compatibility_older_release_preserves_core_selects_fallback_and_disables_newer_feature() {
        let catalog = CapabilityCatalog::builtin();
        let mut environment = future_environment();
        environment.bitbake_version =
            AuthoritativeValue::detected("1.52.0".into(), IdentityAuthority::BitBakeVersionProbe);
        environment.oe_core = AuthoritativeValue::detected(
            ReleaseIdentity {
                name: Some("honister".into()),
                version: Some("3.4".into()),
            },
            IdentityAuthority::ReleaseMetadata,
        );
        let observations = BTreeMap::from([
            (
                CapabilityId::BitBakeWorkspaceInspection,
                vec![observation(CapabilityProbeStatus::Positive, "workspace")],
            ),
            (
                CapabilityId::DevtoolUpgrade,
                vec![observation(CapabilityProbeStatus::Negative, "upgrade")],
            ),
        ]);
        let resolved = CapabilityResolver::default()
            .resolve_snapshot(2, environment, &catalog, &observations)
            .unwrap();
        assert!(
            resolved
                .snapshot
                .allows(CapabilityId::BitBakeWorkspaceInspection)
        );
        assert!(resolved.snapshot.allows(CapabilityId::BitBakeNativeEvents));
        assert!(!resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
        assert_eq!(
            resolved
                .implementations
                .get(&CapabilityId::BitBakeNativeEvents)
                .unwrap()
                .id,
            "tinfoil.adapter.legacy"
        );
        assert!(matches!(
            resolved
                .snapshot
                .capability(CapabilityId::BitBakeNativeEvents)
                .unwrap()
                .state,
            CapabilityState::AvailableWithLimitations { .. }
        ));
        assert!(matches!(
            resolved
                .snapshot
                .capability(CapabilityId::DevtoolUpgrade)
                .unwrap()
                .state,
            CapabilityState::Unavailable { .. }
        ));
        assert_eq!(
            resolved.snapshot.capabilities.len(),
            CapabilityId::ALL.len()
        );
    }
}
