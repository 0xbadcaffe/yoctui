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
            [
                ("bitbake", "bitbake"),
                ("bitbake-getvar", "bitbake-getvar"),
                ("bitbake-diffsigs", "bitbake-diffsigs"),
                ("bitbake-dumpsig", "bitbake-dumpsig"),
                ("devtool", "devtool"),
                ("recipetool", "recipetool"),
                ("bitbake-layers", "bitbake-layers"),
                ("oe-pkgdata-util", "oe-pkgdata-util"),
            ]
            .into_iter()
            .map(|(id, executable)| ToolIdentity {
                id: id.into(),
                executable: format!("{root}/bin/{executable}").into(),
                version: (id == "bitbake")
                    .then(|| bitbake_version.map(str::to_owned))
                    .flatten(),
            })
            .collect(),
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

fn fixture_command_evidence(
    outcome: CapabilityEvidenceOutcome,
    subject: &str,
) -> CapabilityEvidence {
    CapabilityEvidence {
        kind: CapabilityEvidenceKind::DirectProbe,
        outcome,
        subject: subject.into(),
        detail: "deterministic command-surface fixture observation".into(),
        argv: vec!["fixture-help-probe".into()],
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
