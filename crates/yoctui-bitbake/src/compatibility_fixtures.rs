use std::collections::BTreeMap;

use yoctui_model::{
    AuthoritativeValue, BackendIdentity, CapabilityCatalog, CapabilityEvidence,
    CapabilityEvidenceKind, CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
    CapabilityState, DistroIdentity, IdentityAuthority, LayerSeriesIdentity, ProtocolIdentity,
    ReleaseIdentity, SourceRootIdentity, SourceRootKind, ToolIdentity, YoctoEnvironmentIdentity,
};

use crate::{
    CapabilityProbeObservation, CapabilityProbeStatus, CapabilityResolver,
    ResolvedCapabilitySnapshot,
};

/// Policy role only. These labels never claim that the represented release is supported or live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilityFixtureRole {
    OldestPolicyCandidate,
    IntermediateRepresentative,
    CurrentStableCandidate,
    LatestSupportCandidate,
    FutureUnknown,
}

impl CompatibilityFixtureRole {
    pub const ALL: [Self; 5] = [
        Self::OldestPolicyCandidate,
        Self::IntermediateRepresentative,
        Self::CurrentStableCandidate,
        Self::LatestSupportCandidate,
        Self::FutureUnknown,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OldestPolicyCandidate => "oldest-policy-candidate",
            Self::IntermediateRepresentative => "intermediate-representative",
            Self::CurrentStableCandidate => "current-stable-candidate",
            Self::LatestSupportCandidate => "latest-support-candidate",
            Self::FutureUnknown => "future-unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureCapabilityState {
    Available,
    Limited,
    Unavailable,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureCapabilityExpectation {
    pub id: CapabilityId,
    pub state: FixtureCapabilityState,
    pub implementation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseCapabilityFixture {
    pub role: CompatibilityFixtureRole,
    pub fixture_only: bool,
    pub evidence_level: &'static str,
    pub environment: YoctoEnvironmentIdentity,
    pub observations: BTreeMap<CapabilityId, Vec<CapabilityProbeObservation>>,
    pub expectations: Vec<FixtureCapabilityExpectation>,
}

impl ReleaseCapabilityFixture {
    pub fn resolve(&self, generation: u64) -> ResolvedCapabilitySnapshot {
        CapabilityResolver::default()
            .resolve_snapshot(
                generation,
                self.environment.clone(),
                &CapabilityCatalog::builtin(),
                &self.observations,
            )
            .expect("static compatibility fixture must normalize")
    }
}

pub fn release_capability_fixtures() -> Vec<ReleaseCapabilityFixture> {
    let catalog = CapabilityCatalog::builtin();
    let preferred = |id| {
        catalog
            .entry(id)
            .expect("fixture capability must be cataloged")
            .preferred
            .id
            .clone()
    };
    vec![
        ReleaseCapabilityFixture {
            role: CompatibilityFixtureRole::OldestPolicyCandidate,
            fixture_only: true,
            evidence_level: "deterministic_fixture_only",
            environment: fixture_environment(
                CompatibilityFixtureRole::OldestPolicyCandidate,
                Some("1.46.0"),
                None,
            ),
            observations: BTreeMap::from([
                (
                    CapabilityId::DevtoolUpgrade,
                    vec![observation(
                        CapabilityProbeStatus::Negative,
                        "devtool upgrade",
                    )],
                ),
                (
                    CapabilityId::ResultTool,
                    vec![observation(CapabilityProbeStatus::Negative, "resulttool")],
                ),
            ]),
            expectations: vec![
                expectation(
                    CapabilityId::BitBakeWorkspaceInspection,
                    FixtureCapabilityState::Limited,
                    Some("tinfoil.adapter.legacy"),
                ),
                expectation(
                    CapabilityId::BitBakeBuild,
                    FixtureCapabilityState::Limited,
                    Some("tinfoil.adapter.legacy"),
                ),
                expectation(
                    CapabilityId::DevtoolUpgrade,
                    FixtureCapabilityState::Unavailable,
                    None,
                ),
                expectation(
                    CapabilityId::ResultTool,
                    FixtureCapabilityState::Unavailable,
                    None,
                ),
            ],
        },
        ReleaseCapabilityFixture {
            role: CompatibilityFixtureRole::IntermediateRepresentative,
            fixture_only: true,
            evidence_level: "deterministic_fixture_only",
            environment: fixture_environment(
                CompatibilityFixtureRole::IntermediateRepresentative,
                Some("2.8.0"),
                None,
            ),
            observations: BTreeMap::from([(
                CapabilityId::DevtoolUpgrade,
                vec![observation(
                    CapabilityProbeStatus::Positive,
                    "devtool upgrade",
                )],
            )]),
            expectations: vec![
                expectation(
                    CapabilityId::BitBakeWorkspaceInspection,
                    FixtureCapabilityState::Limited,
                    Some("tinfoil.adapter.modern"),
                ),
                expectation(
                    CapabilityId::BitBakeBuild,
                    FixtureCapabilityState::Limited,
                    Some("tinfoil.adapter.modern"),
                ),
                expectation(
                    CapabilityId::DevtoolUpgrade,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::DevtoolUpgrade)),
                ),
            ],
        },
        ReleaseCapabilityFixture {
            role: CompatibilityFixtureRole::CurrentStableCandidate,
            fixture_only: true,
            evidence_level: "deterministic_fixture_only",
            environment: fixture_environment(
                CompatibilityFixtureRole::CurrentStableCandidate,
                Some("2.18.0"),
                None,
            ),
            observations: BTreeMap::from([
                (
                    CapabilityId::BitBakeWorkspaceInspection,
                    vec![observation(CapabilityProbeStatus::Positive, "workspace")],
                ),
                (
                    CapabilityId::ResultTool,
                    vec![observation(CapabilityProbeStatus::Positive, "resulttool")],
                ),
            ]),
            expectations: vec![
                expectation(
                    CapabilityId::BitBakeWorkspaceInspection,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::BitBakeWorkspaceInspection)),
                ),
                expectation(
                    CapabilityId::BitBakeBuild,
                    FixtureCapabilityState::Limited,
                    Some("tinfoil.adapter.modern"),
                ),
                expectation(
                    CapabilityId::ResultTool,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::ResultTool)),
                ),
            ],
        },
        ReleaseCapabilityFixture {
            role: CompatibilityFixtureRole::LatestSupportCandidate,
            fixture_only: true,
            evidence_level: "deterministic_fixture_only",
            environment: fixture_environment(
                CompatibilityFixtureRole::LatestSupportCandidate,
                Some("2.19.0"),
                Some("6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250"),
            ),
            observations: BTreeMap::from([
                (
                    CapabilityId::BitBakeWorkspaceInspection,
                    vec![observation(CapabilityProbeStatus::Positive, "workspace")],
                ),
                (
                    CapabilityId::DevtoolUpgrade,
                    vec![observation(
                        CapabilityProbeStatus::Positive,
                        "devtool upgrade",
                    )],
                ),
            ]),
            expectations: vec![
                expectation(
                    CapabilityId::BitBakeWorkspaceInspection,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::BitBakeWorkspaceInspection)),
                ),
                expectation(
                    CapabilityId::BitBakeBuild,
                    FixtureCapabilityState::Unknown,
                    None,
                ),
                expectation(
                    CapabilityId::DevtoolUpgrade,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::DevtoolUpgrade)),
                ),
            ],
        },
        ReleaseCapabilityFixture {
            role: CompatibilityFixtureRole::FutureUnknown,
            fixture_only: true,
            evidence_level: "deterministic_fixture_only",
            environment: fixture_environment(
                CompatibilityFixtureRole::FutureUnknown,
                Some("99.0.0"),
                Some("fixture-future-unknown"),
            ),
            observations: BTreeMap::from([
                (
                    CapabilityId::DevtoolUpgrade,
                    vec![observation(
                        CapabilityProbeStatus::Positive,
                        "devtool upgrade",
                    )],
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
            ]),
            expectations: vec![
                expectation(
                    CapabilityId::DevtoolUpgrade,
                    FixtureCapabilityState::Available,
                    Some(&preferred(CapabilityId::DevtoolUpgrade)),
                ),
                expectation(
                    CapabilityId::ResultTool,
                    FixtureCapabilityState::Unavailable,
                    None,
                ),
                expectation(
                    CapabilityId::WicCreate,
                    FixtureCapabilityState::Unknown,
                    None,
                ),
                expectation(CapabilityId::RunQemu, FixtureCapabilityState::Unknown, None),
                expectation(
                    CapabilityId::BitBakeBuild,
                    FixtureCapabilityState::Unknown,
                    None,
                ),
            ],
        },
    ]
}

fn fixture_environment(
    role: CompatibilityFixtureRole,
    bitbake_version: Option<&str>,
    observed_release: Option<&str>,
) -> YoctoEnvironmentIdentity {
    let root = format!("/fixtures/{}", role.as_str());
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            format!("{root}/build").into(),
            IdentityAuthority::InitializedEnvironment,
        ),
        source_roots: AuthoritativeValue::detected(
            vec![SourceRootIdentity {
                kind: SourceRootKind::Other("fixture-source".into()),
                path: root.clone().into(),
            }],
            IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: bitbake_version.map_or(AuthoritativeValue::Unknown, |version| {
            AuthoritativeValue::detected(version.into(), IdentityAuthority::BitBakeVersionProbe)
        }),
        oe_core: observed_release.map_or(AuthoritativeValue::Unknown, |version| {
            AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: None,
                    version: Some(version.into()),
                },
                IdentityAuthority::ReleaseMetadata,
            )
        }),
        poky: AuthoritativeValue::Unknown,
        distro: AuthoritativeValue::detected(
            DistroIdentity {
                name: "poky".into(),
                version: None,
            },
            IdentityAuthority::BitBakeDatastore,
        ),
        machine: AuthoritativeValue::detected(
            "qemux86-64".into(),
            IdentityAuthority::BitBakeDatastore,
        ),
        layer_series: AuthoritativeValue::detected(
            vec![LayerSeriesIdentity {
                layer: "fixture-core".into(),
                root: format!("{root}/meta").into(),
                compatible_series: vec![role.as_str().into()],
            }],
            IdentityAuthority::ConfiguredLayerMetadata,
        ),
        available_tools: AuthoritativeValue::detected(
            vec![ToolIdentity {
                id: "bitbake".into(),
                executable: format!("{root}/bin/bitbake").into(),
                version: bitbake_version.map(str::to_owned),
            }],
            IdentityAuthority::ExecutableProbe,
        ),
        backend: AuthoritativeValue::detected(
            BackendIdentity {
                name: "fixture-tinfoil".into(),
                version: bitbake_version.map(str::to_owned),
            },
            IdentityAuthority::BackendHandshake,
        ),
        protocol: AuthoritativeValue::detected(
            ProtocolIdentity {
                name: "yoctui-daemon".into(),
                version: "1.0".into(),
            },
            IdentityAuthority::ProtocolNegotiation,
        ),
    }
}

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
            detail: format!("deterministic fixture observation: {subject}"),
            argv: Vec::new(),
        },
    }
}

fn expectation(
    id: CapabilityId,
    state: FixtureCapabilityState,
    implementation: Option<&str>,
) -> FixtureCapabilityExpectation {
    FixtureCapabilityExpectation {
        id,
        state,
        implementation: implementation.map(str::to_owned),
    }
}

pub fn fixture_state(state: &CapabilityState) -> FixtureCapabilityState {
    match state {
        CapabilityState::Available => FixtureCapabilityState::Available,
        CapabilityState::AvailableWithLimitations { .. } => FixtureCapabilityState::Limited,
        CapabilityState::Unavailable { .. } => FixtureCapabilityState::Unavailable,
        CapabilityState::Unknown { .. } => FixtureCapabilityState::Unknown,
        CapabilityState::Unsupported { .. } => FixtureCapabilityState::Unsupported,
    }
}

pub fn fixture_implementation(
    snapshot: &ResolvedCapabilitySnapshot,
    id: CapabilityId,
) -> Option<&CapabilityImplementation> {
    snapshot.implementations.get(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_fixture_catalog_has_five_unclaimed_exact_identities() {
        let fixtures = release_capability_fixtures();
        assert_eq!(fixtures.len(), CompatibilityFixtureRole::ALL.len());
        for (fixture, role) in fixtures.iter().zip(CompatibilityFixtureRole::ALL) {
            assert_eq!(fixture.role, role);
            assert!(fixture.fixture_only);
            assert_eq!(fixture.evidence_level, "deterministic_fixture_only");
            assert!(
                fixture
                    .environment
                    .build_directory
                    .value()
                    .unwrap()
                    .is_absolute()
            );
            assert_eq!(
                fixture.environment.machine.value().map(String::as_str),
                Some("qemux86-64")
            );
            assert_eq!(fixture.environment.protocol.value().unwrap().version, "1.0");
        }
    }

    #[test]
    fn compatibility_fixture_capability_differences_are_exact_and_complete() {
        for (index, fixture) in release_capability_fixtures().iter().enumerate() {
            let resolved = fixture.resolve(index as u64 + 1);
            assert_eq!(
                resolved.snapshot.capabilities.len(),
                CapabilityId::ALL.len()
            );
            for expected in &fixture.expectations {
                let record = resolved.snapshot.capability(expected.id).unwrap();
                assert_eq!(
                    fixture_state(&record.state),
                    expected.state,
                    "{} {}",
                    fixture.role.as_str(),
                    expected.id.as_str()
                );
                assert_eq!(
                    fixture_implementation(&resolved, expected.id).map(|value| value.id.as_str()),
                    expected.implementation.as_deref(),
                    "{} {}",
                    fixture.role.as_str(),
                    expected.id.as_str()
                );
            }
        }
    }

    #[test]
    fn compatibility_fixture_future_enables_only_positive_direct_observations() {
        let fixture = release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == CompatibilityFixtureRole::FutureUnknown)
            .unwrap();
        let resolved = fixture.resolve(99);
        assert!(resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
        for id in [
            CapabilityId::BitBakeBuild,
            CapabilityId::ResultTool,
            CapabilityId::WicCreate,
            CapabilityId::RunQemu,
        ] {
            assert!(!resolved.snapshot.allows(id), "{}", id.as_str());
        }
        assert!(
            resolved
                .snapshot
                .capability(CapabilityId::BitBakeBuild)
                .unwrap()
                .evidence
                .iter()
                .all(|evidence| evidence.outcome != CapabilityEvidenceOutcome::Positive)
        );
    }
}
