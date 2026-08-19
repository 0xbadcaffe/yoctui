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
mod tests {
    use super::*;
    use crate::CapabilityProbeStatus;
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

    fn complete_observations(
        catalog: &CapabilityCatalog,
        id: CapabilityId,
        status: CapabilityProbeStatus,
        subject: &str,
    ) -> Vec<CapabilityProbeObservation> {
        vec![observation(status, subject); catalog.entry(id).unwrap().probes.len()]
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
                complete_observations(
                    &catalog,
                    CapabilityId::DevtoolUpgrade,
                    CapabilityProbeStatus::Positive,
                    "upgrade",
                ),
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
    fn compatibility_probe_aggregation_keeps_partial_compound_evidence_disabled() {
        let catalog = CapabilityCatalog::builtin();
        let entry = catalog.entry(CapabilityId::DevtoolUpgrade).unwrap();
        let resolver = CapabilityResolver::default();

        let complete = complete_observations(
            &catalog,
            entry.id,
            CapabilityProbeStatus::Positive,
            "devtool upgrade",
        );
        let available = resolver.resolve(entry, Some("2.18"), &complete);
        assert_eq!(available.record.state, CapabilityState::Available);
        assert_eq!(available.implementation, Some(entry.preferred.clone()));

        let partial = vec![
            observation(CapabilityProbeStatus::Positive, "devtool executable"),
            observation(
                CapabilityProbeStatus::Inconclusive,
                "upgrade --help timeout",
            ),
        ];
        let unknown = resolver.resolve(entry, Some("2.18"), &partial);
        assert!(matches!(
            unknown.record.state,
            CapabilityState::Unknown { ref reason }
                if reason.code.as_str() == "evidence.incomplete"
        ));
        assert!(unknown.implementation.is_none());

        let unavailable = resolver.resolve(
            entry,
            Some("2.18"),
            &[
                observation(CapabilityProbeStatus::Negative, "upgrade absent"),
                observation(CapabilityProbeStatus::Inconclusive, "help timeout"),
            ],
        );
        assert!(matches!(
            unavailable.record.state,
            CapabilityState::Unavailable { .. }
        ));
        assert!(unavailable.implementation.is_none());

        let conflict = resolver.resolve(
            entry,
            Some("2.18"),
            &[
                observation(CapabilityProbeStatus::Positive, "upgrade present"),
                observation(CapabilityProbeStatus::Negative, "upgrade absent"),
            ],
        );
        assert!(matches!(
            conflict.record.state,
            CapabilityState::Unknown { ref reason }
                if reason.code.as_str() == "evidence.conflict"
        ));
        assert!(conflict.implementation.is_none());
    }

    #[test]
    fn compatibility_probe_aggregation_allows_fallback_only_after_complete_inconclusive_probe() {
        let catalog = CapabilityCatalog::builtin();
        let entry = catalog.entry(CapabilityId::BitBakeBuild).unwrap();
        let resolver = CapabilityResolver::default();

        let missing = resolver.resolve(entry, Some("2.18"), &[]);
        assert!(matches!(
            missing.record.state,
            CapabilityState::Unknown { ref reason }
                if reason.code.as_str() == "evidence.missing"
        ));
        assert!(missing.implementation.is_none());

        let probed = resolver.resolve(
            entry,
            Some("2.18"),
            &[observation(
                CapabilityProbeStatus::Inconclusive,
                "backend handshake unavailable",
            )],
        );
        assert!(matches!(
            probed.record.state,
            CapabilityState::AvailableWithLimitations { .. }
        ));
        assert_eq!(probed.implementation.unwrap().id, "tinfoil.adapter.modern");
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
            (
                CapabilityId::BitBakeNativeEvents,
                vec![observation(
                    CapabilityProbeStatus::Inconclusive,
                    "native events backend unprobeable",
                )],
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

    #[test]
    fn compatibility_command_getvar_prefers_direct_utility_and_uses_environment_capability_fallback()
     {
        let catalog = CapabilityCatalog::builtin();
        let positive_getvar = BTreeMap::from([(
            CapabilityId::BitBakeGetVar,
            complete_observations(
                &catalog,
                CapabilityId::BitBakeGetVar,
                CapabilityProbeStatus::Positive,
                "bitbake-getvar help and required options",
            ),
        )]);
        let direct = CapabilityResolver::default()
            .resolve_snapshot(3, future_environment(), &catalog, &positive_getvar)
            .unwrap();
        assert_eq!(
            direct
                .implementations
                .get(&CapabilityId::BitBakeGetVar)
                .unwrap()
                .id,
            "bitbake_getvar.argv"
        );
        assert_eq!(
            direct
                .snapshot
                .capability(CapabilityId::BitBakeGetVar)
                .unwrap()
                .state,
            CapabilityState::Available
        );

        let environment_only = BTreeMap::from([(
            CapabilityId::BitBakeEnvironmentDump,
            complete_observations(
                &catalog,
                CapabilityId::BitBakeEnvironmentDump,
                CapabilityProbeStatus::Positive,
                "bitbake -e",
            ),
        )]);
        let fallback = CapabilityResolver::default()
            .resolve_snapshot(4, future_environment(), &catalog, &environment_only)
            .unwrap();
        assert_eq!(
            fallback
                .implementations
                .get(&CapabilityId::BitBakeGetVar)
                .unwrap()
                .id,
            "bitbake.environment_lookup"
        );
        assert!(matches!(
            fallback
                .snapshot
                .capability(CapabilityId::BitBakeGetVar)
                .unwrap()
                .state,
            CapabilityState::AvailableWithLimitations { .. }
        ));
    }
}
