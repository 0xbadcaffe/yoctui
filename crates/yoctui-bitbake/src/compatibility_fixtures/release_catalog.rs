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
    let complete = |id, status, subject: &str| {
        vec![
            observation(status, subject);
            catalog
                .entry(id)
                .expect("fixture capability must be cataloged")
                .probes
                .len()
        ]
    };
    let conflicting = |id, subject: &str| {
        let mut observations = complete(id, CapabilityProbeStatus::Positive, subject);
        observations
            .last_mut()
            .expect("catalog probes are nonempty")
            .status = CapabilityProbeStatus::Negative;
        observations
            .last_mut()
            .expect("catalog probes are nonempty")
            .evidence
            .outcome = CapabilityEvidenceOutcome::Negative;
        observations
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
                    CapabilityId::BitBakeWorkspaceInspection,
                    complete(
                        CapabilityId::BitBakeWorkspaceInspection,
                        CapabilityProbeStatus::Inconclusive,
                        "workspace backend unprobeable",
                    ),
                ),
                (
                    CapabilityId::BitBakeBuild,
                    complete(
                        CapabilityId::BitBakeBuild,
                        CapabilityProbeStatus::Inconclusive,
                        "build backend unprobeable",
                    ),
                ),
                (
                    CapabilityId::DevtoolUpgrade,
                    complete(
                        CapabilityId::DevtoolUpgrade,
                        CapabilityProbeStatus::Negative,
                        "devtool upgrade",
                    ),
                ),
                (
                    CapabilityId::ResultTool,
                    complete(
                        CapabilityId::ResultTool,
                        CapabilityProbeStatus::Negative,
                        "resulttool",
                    ),
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
            observations: BTreeMap::from([
                (
                    CapabilityId::BitBakeWorkspaceInspection,
                    complete(
                        CapabilityId::BitBakeWorkspaceInspection,
                        CapabilityProbeStatus::Inconclusive,
                        "workspace backend unprobeable",
                    ),
                ),
                (
                    CapabilityId::BitBakeBuild,
                    complete(
                        CapabilityId::BitBakeBuild,
                        CapabilityProbeStatus::Inconclusive,
                        "build backend unprobeable",
                    ),
                ),
                (
                    CapabilityId::DevtoolUpgrade,
                    complete(
                        CapabilityId::DevtoolUpgrade,
                        CapabilityProbeStatus::Positive,
                        "devtool upgrade",
                    ),
                ),
            ]),
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
                    complete(
                        CapabilityId::BitBakeWorkspaceInspection,
                        CapabilityProbeStatus::Positive,
                        "workspace",
                    ),
                ),
                (
                    CapabilityId::ResultTool,
                    complete(
                        CapabilityId::ResultTool,
                        CapabilityProbeStatus::Positive,
                        "resulttool",
                    ),
                ),
                (
                    CapabilityId::BitBakeBuild,
                    complete(
                        CapabilityId::BitBakeBuild,
                        CapabilityProbeStatus::Inconclusive,
                        "build backend unprobeable",
                    ),
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
                    complete(
                        CapabilityId::BitBakeWorkspaceInspection,
                        CapabilityProbeStatus::Positive,
                        "workspace",
                    ),
                ),
                (
                    CapabilityId::DevtoolUpgrade,
                    complete(
                        CapabilityId::DevtoolUpgrade,
                        CapabilityProbeStatus::Positive,
                        "devtool upgrade",
                    ),
                ),
                (
                    CapabilityId::BitBakeBuild,
                    complete(
                        CapabilityId::BitBakeBuild,
                        CapabilityProbeStatus::Inconclusive,
                        "build backend unprobeable",
                    ),
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
                    complete(
                        CapabilityId::DevtoolUpgrade,
                        CapabilityProbeStatus::Positive,
                        "devtool upgrade",
                    ),
                ),
                (
                    CapabilityId::ResultTool,
                    complete(
                        CapabilityId::ResultTool,
                        CapabilityProbeStatus::Negative,
                        "resulttool",
                    ),
                ),
                (
                    CapabilityId::WicCreate,
                    conflicting(CapabilityId::WicCreate, "wic create conflict"),
                ),
                (
                    CapabilityId::BitBakeBuild,
                    complete(
                        CapabilityId::BitBakeBuild,
                        CapabilityProbeStatus::Inconclusive,
                        "build backend unprobeable",
                    ),
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
