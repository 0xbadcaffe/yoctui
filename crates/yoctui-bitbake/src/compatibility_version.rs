use std::cmp::Ordering;

use thiserror::Error;
use yoctui_model::{
    CapabilityCatalogEntry, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityReason,
    CapabilityState, FallbackSelector,
};

use crate::{CapabilityProbeObservation, CapabilityProbeStatus};

const TINFOIL_MAP_KEY: &str = "bitbake.tinfoil_adapter";
const BITBAKE_RELEASE_MANUALS: &str = "https://docs.yoctoproject.org/bitbake/releases.html";
const KIRKSTONE_RELEASE_NOTES: &str =
    "https://docs.yoctoproject.org/4.0.4/migration-guides/release-notes-4.0.html";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorrelatedVersion {
    components: Vec<u64>,
}

impl CorrelatedVersion {
    pub fn parse(value: &str) -> Result<Self, VersionParseError> {
        let numeric = value
            .split_once(['+', '-'])
            .map_or(value, |(numeric, _)| numeric);
        let mut components = numeric
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| VersionParseError(value.into()))?;
        if components.is_empty()
            || components.len() > 4
            || numeric.is_empty()
            || numeric.starts_with('.')
            || numeric.ends_with('.')
        {
            return Err(VersionParseError(value.into()));
        }
        components.resize(4, 0);
        Ok(Self { components })
    }

    fn component(&self, index: usize) -> u64 {
        self.components.get(index).copied().unwrap_or(0)
    }
}

impl PartialOrd for CorrelatedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CorrelatedVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        (0..4)
            .map(|index| self.component(index).cmp(&other.component(index)))
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    }
}

impl std::fmt::Display for CorrelatedVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let last = self
            .components
            .iter()
            .rposition(|component| *component != 0)
            .unwrap_or(0);
        let text = self.components[..=last]
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(".");
        formatter.write_str(&text)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("unrecognized correlated version: {0}")]
pub struct VersionParseError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFallbackResolution {
    Direct {
        outcome: CapabilityEvidenceOutcome,
    },
    Inferred {
        implementation: CapabilityImplementation,
        state: CapabilityState,
        evidence: CapabilityEvidence,
    },
    Unknown {
        state: CapabilityState,
        evidence: CapabilityEvidence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectEvidenceAssessment {
    CompletePositive,
    Negative,
    CompleteInconclusive,
    Conflict,
    Incomplete,
    Missing,
}

pub(crate) fn assess_direct_evidence(
    entry: &CapabilityCatalogEntry,
    direct: &[CapabilityProbeObservation],
) -> DirectEvidenceAssessment {
    if direct.is_empty() {
        return DirectEvidenceAssessment::Missing;
    }
    let positive = direct
        .iter()
        .any(|observation| observation.status == CapabilityProbeStatus::Positive);
    let negative = direct
        .iter()
        .any(|observation| observation.status == CapabilityProbeStatus::Negative);
    let inconclusive = direct
        .iter()
        .any(|observation| observation.status == CapabilityProbeStatus::Inconclusive);
    if positive && negative {
        DirectEvidenceAssessment::Conflict
    } else if negative {
        DirectEvidenceAssessment::Negative
    } else if direct.len() != entry.probes.len() || positive && inconclusive {
        DirectEvidenceAssessment::Incomplete
    } else if positive {
        DirectEvidenceAssessment::CompletePositive
    } else {
        DirectEvidenceAssessment::CompleteInconclusive
    }
}

#[derive(Debug, Clone)]
struct VersionFallbackRule {
    map_key: &'static str,
    capabilities: &'static [CapabilityId],
    minimum_inclusive: &'static str,
    maximum_exclusive: &'static str,
    implementation: &'static str,
    official_sources: &'static [&'static str],
    limitation: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct VersionFallbackMap;

impl VersionFallbackMap {
    pub fn resolve_bitbake(
        &self,
        entry: &CapabilityCatalogEntry,
        version: Option<&str>,
        direct: &[CapabilityProbeObservation],
    ) -> VersionFallbackResolution {
        match assess_direct_evidence(entry, direct) {
            DirectEvidenceAssessment::CompletePositive => {
                return VersionFallbackResolution::Direct {
                    outcome: CapabilityEvidenceOutcome::Positive,
                };
            }
            DirectEvidenceAssessment::Negative => {
                return VersionFallbackResolution::Direct {
                    outcome: CapabilityEvidenceOutcome::Negative,
                };
            }
            DirectEvidenceAssessment::Conflict => {
                return unknown(
                    "evidence.conflict",
                    "Direct capability probes returned conflicting positive and negative evidence.",
                    "direct capability probes conflict",
                );
            }
            DirectEvidenceAssessment::Incomplete => {
                return unknown(
                    "evidence.incomplete",
                    "Required capability probes were incomplete; partial positive evidence cannot enable the feature.",
                    "one or more required direct probes are absent or inconclusive",
                );
            }
            DirectEvidenceAssessment::Missing => {
                return unknown(
                    "evidence.missing",
                    "Required capability probes were not collected.",
                    "no direct capability observations were supplied",
                );
            }
            DirectEvidenceAssessment::CompleteInconclusive => {}
        }

        let Some(map_key) = declared_map_key(entry) else {
            return unknown(
                "fallback.not_declared",
                "This capability does not declare a release/version fallback.",
                "no catalog fallback selector",
            );
        };
        let Some(raw_version) = version else {
            return unknown(
                "version.unknown",
                "BitBake version is unknown and direct capability probing was inconclusive.",
                "BitBake version is unavailable",
            );
        };
        let parsed = match CorrelatedVersion::parse(raw_version) {
            Ok(parsed) => parsed,
            Err(_) => {
                return unknown(
                    "version.malformed",
                    "BitBake version is unrecognized and direct capability probing was inconclusive.",
                    &format!("unrecognized BitBake version {raw_version}"),
                );
            }
        };
        let rule = rules().into_iter().find(|rule| {
            rule.map_key == map_key
                && rule.capabilities.contains(&entry.id)
                && in_range(&parsed, rule.minimum_inclusive, rule.maximum_exclusive)
        });
        let Some(rule) = rule else {
            return unknown(
                "version.outside_fallback_map",
                "BitBake version is outside the documented fallback map; positive probing is required.",
                &format!("no conservative fallback rule for BitBake {parsed}"),
            );
        };
        let source_summary = rule.official_sources.join(", ");
        VersionFallbackResolution::Inferred {
            implementation: CapabilityImplementation {
                id: rule.implementation.into(),
                kind: CapabilityImplementationKind::BackendApi,
            },
            state: CapabilityState::AvailableWithLimitations {
                reason: CapabilityReason::new(
                    "fallback.version_inference",
                    "Direct probing was inconclusive; a centralized BitBake adapter fallback was selected.",
                    Some(format!("Official correlation sources: {source_summary}")),
                )
                .expect("static fallback reason must be valid"),
                limitations: vec![rule.limitation.into()],
            },
            evidence: CapabilityEvidence {
                kind: CapabilityEvidenceKind::ReleaseVersionFallback,
                outcome: CapabilityEvidenceOutcome::Positive,
                subject: format!("BitBake {parsed}"),
                detail: format!(
                    "Selected {} for the documented range [{}..{}); direct probes remain authoritative",
                    rule.implementation, rule.minimum_inclusive, rule.maximum_exclusive
                ),
                argv: Vec::new(),
            },
        }
    }
}

fn declared_map_key(entry: &CapabilityCatalogEntry) -> Option<&str> {
    match entry.fallback.as_ref().map(|fallback| &fallback.selector) {
        Some(FallbackSelector::VersionInferenceWhenUnprobeable { map_key }) => Some(map_key),
        _ => None,
    }
}

fn in_range(version: &CorrelatedVersion, minimum: &str, maximum: &str) -> bool {
    let minimum = CorrelatedVersion::parse(minimum).expect("static minimum version must parse");
    let maximum = CorrelatedVersion::parse(maximum).expect("static maximum version must parse");
    version >= &minimum && version < &maximum
}

fn rules() -> [VersionFallbackRule; 2] {
    const TINFOIL_CAPABILITIES: &[CapabilityId] = &[
        CapabilityId::BitBakeWorkspaceInspection,
        CapabilityId::BitBakeRecipeInventory,
        CapabilityId::BitBakeRecipeDependencies,
        CapabilityId::BitBakeRecipeSources,
        CapabilityId::BitBakeRecipeMetadata,
        CapabilityId::BitBakeLayerInventory,
        CapabilityId::BitBakeLayerRelationships,
        CapabilityId::BitBakeBuild,
        CapabilityId::BitBakeCancellation,
        CapabilityId::BitBakeTaskList,
        CapabilityId::BitBakeServerSocket,
        CapabilityId::BitBakeNativeEvents,
    ];
    [
        VersionFallbackRule {
            map_key: TINFOIL_MAP_KEY,
            capabilities: TINFOIL_CAPABILITIES,
            minimum_inclusive: "1.46",
            maximum_exclusive: "2.0",
            implementation: "tinfoil.adapter.legacy",
            official_sources: &[BITBAKE_RELEASE_MANUALS],
            limitation: "Legacy Tinfoil API family is inferred from BitBake 1.46–1.52 only when backend negotiation cannot directly identify the API.",
        },
        VersionFallbackRule {
            map_key: TINFOIL_MAP_KEY,
            capabilities: TINFOIL_CAPABILITIES,
            minimum_inclusive: "2.0",
            maximum_exclusive: "2.19",
            implementation: "tinfoil.adapter.modern",
            official_sources: &[BITBAKE_RELEASE_MANUALS, KIRKSTONE_RELEASE_NOTES],
            limitation: "Modern Tinfoil API family is inferred only through the latest documented BitBake 2.18 boundary; direct negotiation is still preferred.",
        },
    ]
}

fn unknown(code: &str, message: &str, detail: &str) -> VersionFallbackResolution {
    VersionFallbackResolution::Unknown {
        state: CapabilityState::Unknown {
            reason: CapabilityReason::new(code, message, None)
                .expect("static unknown reason must be valid"),
        },
        evidence: CapabilityEvidence {
            kind: CapabilityEvidenceKind::ReleaseVersionFallback,
            outcome: CapabilityEvidenceOutcome::Inconclusive,
            subject: "BitBake version fallback".into(),
            detail: detail.into(),
            argv: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoctui_model::{CapabilityCatalog, CapabilityProbeSpec};

    fn entry(id: CapabilityId) -> CapabilityCatalogEntry {
        CapabilityCatalog::builtin().entry(id).unwrap().clone()
    }

    fn direct(status: CapabilityProbeStatus) -> CapabilityProbeObservation {
        CapabilityProbeObservation {
            status,
            evidence: CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: match status {
                    CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                    CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                    CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
                },
                subject: "backend handshake".into(),
                detail: "fixture observation".into(),
                argv: Vec::new(),
            },
        }
    }

    #[test]
    fn compatibility_version_parser_compares_numeric_components_and_suffixes() {
        assert!(
            CorrelatedVersion::parse("1.52").unwrap() < CorrelatedVersion::parse("2.0").unwrap()
        );
        assert_eq!(
            CorrelatedVersion::parse("2.8").unwrap(),
            CorrelatedVersion::parse("2.8.0").unwrap()
        );
        assert_eq!(
            CorrelatedVersion::parse("2.18.0+git").unwrap(),
            CorrelatedVersion::parse("2.18").unwrap()
        );
        for malformed in ["", ".2", "2.", "two", "2..1", "1.2.3.4.5"] {
            assert!(CorrelatedVersion::parse(malformed).is_err(), "{malformed}");
        }
    }

    #[test]
    fn compatibility_version_selects_old_and_new_adapter_ranges_only_when_declared() {
        let map = VersionFallbackMap;
        let old = map.resolve_bitbake(
            &entry(CapabilityId::BitBakeWorkspaceInspection),
            Some("1.52.0"),
            &[direct(CapabilityProbeStatus::Inconclusive)],
        );
        assert!(matches!(
            old,
            VersionFallbackResolution::Inferred { implementation, .. }
                if implementation.id == "tinfoil.adapter.legacy"
        ));
        let new = map.resolve_bitbake(
            &entry(CapabilityId::BitBakeWorkspaceInspection),
            Some("2.18.0"),
            &[direct(CapabilityProbeStatus::Inconclusive)],
        );
        assert!(matches!(
            new,
            VersionFallbackResolution::Inferred { implementation, .. }
                if implementation.id == "tinfoil.adapter.modern"
        ));
        let undeclared =
            map.resolve_bitbake(&entry(CapabilityId::DevtoolUpgrade), Some("2.18"), &[]);
        assert!(matches!(
            undeclared,
            VersionFallbackResolution::Unknown { .. }
        ));
    }

    #[test]
    fn compatibility_version_direct_evidence_overrides_static_inference() {
        let map = VersionFallbackMap;
        for (status, outcome) in [
            (
                CapabilityProbeStatus::Positive,
                CapabilityEvidenceOutcome::Positive,
            ),
            (
                CapabilityProbeStatus::Negative,
                CapabilityEvidenceOutcome::Negative,
            ),
        ] {
            assert_eq!(
                map.resolve_bitbake(
                    &entry(CapabilityId::BitBakeNativeEvents),
                    Some("1.52"),
                    &[direct(status)],
                ),
                VersionFallbackResolution::Direct { outcome }
            );
        }
        assert!(matches!(
            map.resolve_bitbake(
                &entry(CapabilityId::BitBakeNativeEvents),
                Some("2.8"),
                &[
                    direct(CapabilityProbeStatus::Positive),
                    direct(CapabilityProbeStatus::Negative)
                ],
            ),
            VersionFallbackResolution::Unknown { .. }
        ));
    }

    #[test]
    fn compatibility_version_unknown_malformed_and_future_versions_fail_closed() {
        let map = VersionFallbackMap;
        let capability = entry(CapabilityId::BitBakeBuild);
        for version in [
            None,
            Some("future"),
            Some("1.45"),
            Some("2.19"),
            Some("3.0"),
        ] {
            let resolution = map.resolve_bitbake(&capability, version, &[]);
            assert!(
                matches!(resolution, VersionFallbackResolution::Unknown { .. }),
                "{version:?}"
            );
        }
    }

    #[test]
    fn compatibility_probe_aggregation_requires_every_direct_requirement() {
        let map = VersionFallbackMap;
        let capability = entry(CapabilityId::DevtoolUpgrade);
        assert!(capability.probes.len() > 1);

        let complete = vec![direct(CapabilityProbeStatus::Positive); capability.probes.len()];
        assert_eq!(
            map.resolve_bitbake(&capability, Some("2.18"), &complete),
            VersionFallbackResolution::Direct {
                outcome: CapabilityEvidenceOutcome::Positive,
            }
        );
        for incomplete in [
            vec![direct(CapabilityProbeStatus::Positive)],
            vec![
                direct(CapabilityProbeStatus::Positive),
                direct(CapabilityProbeStatus::Inconclusive),
            ],
        ] {
            let resolution = map.resolve_bitbake(&capability, Some("2.18"), &incomplete);
            assert!(matches!(
                resolution,
                VersionFallbackResolution::Unknown {
                    state: CapabilityState::Unknown { ref reason },
                    ..
                } if reason.code.as_str() == "evidence.incomplete"
            ));
        }
        assert!(matches!(
            map.resolve_bitbake(&capability, Some("2.18"), &[]),
            VersionFallbackResolution::Unknown {
                state: CapabilityState::Unknown { ref reason },
                ..
            } if reason.code.as_str() == "evidence.missing"
        ));
        assert_eq!(
            map.resolve_bitbake(
                &capability,
                Some("2.18"),
                &[
                    direct(CapabilityProbeStatus::Negative),
                    direct(CapabilityProbeStatus::Inconclusive),
                ],
            ),
            VersionFallbackResolution::Direct {
                outcome: CapabilityEvidenceOutcome::Negative,
            }
        );
    }

    #[test]
    fn compatibility_version_catalog_fallback_is_not_an_executable_probe() {
        let capability = entry(CapabilityId::BitBakeBuild);
        assert!(
            capability
                .probes
                .iter()
                .all(|probe| !matches!(probe, CapabilityProbeSpec::CommandHelp { .. }))
        );
        let VersionFallbackResolution::Inferred {
            state, evidence, ..
        } = VersionFallbackMap.resolve_bitbake(
            &capability,
            Some("2.8.1"),
            &[direct(CapabilityProbeStatus::Inconclusive)],
        )
        else {
            panic!("expected inferred adapter");
        };
        assert!(matches!(
            state,
            CapabilityState::AvailableWithLimitations { .. }
        ));
        assert_eq!(
            evidence.kind,
            CapabilityEvidenceKind::ReleaseVersionFallback
        );
    }
}
