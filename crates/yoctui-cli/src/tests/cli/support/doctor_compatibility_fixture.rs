pub(crate) fn doctor_compatibility_fixture() -> yoctui_protocol::daemon::CompatibilitySnapshotData {
    use yoctui_protocol::daemon::{
        COMPATIBILITY_SCHEMA_VERSION, CompatibilityBackendIdentity, CompatibilityCapabilityData,
        CompatibilityDetected, CompatibilityDistroIdentity, CompatibilityEnvironmentIdentity,
        CompatibilityEvidenceData, CompatibilityEvidenceKind, CompatibilityEvidenceOutcome,
        CompatibilityIdentityAuthority, CompatibilityImplementationData,
        CompatibilityProtocolIdentity, CompatibilityReasonData, CompatibilityReleaseIdentity,
        CompatibilityStateData, CompatibilityToolIdentity,
    };
    let evidence = |kind, outcome, subject: &str, detail: &str| CompatibilityEvidenceData {
        kind,
        outcome,
        subject: subject.into(),
        detail: detail.into(),
        argv: vec![subject.into(), "--help".into()],
    };
    let reason = |code: &str, message: &str, requirement: &str| CompatibilityReasonData {
        code: code.into(),
        message: message.into(),
        requirement: Some(requirement.into()),
    };
    yoctui_protocol::daemon::CompatibilitySnapshotData {
        schema_version: COMPATIBILITY_SCHEMA_VERSION,
        generation: 12,
        environment: CompatibilityEnvironmentIdentity {
            build_directory: CompatibilityDetected::Detected {
                value: "/work/poky/build".into(),
                authority: CompatibilityIdentityAuthority::InitializedEnvironment,
            },
            source_roots: CompatibilityDetected::Unknown,
            bitbake_version: CompatibilityDetected::Detected {
                value: "2.18.0".into(),
                authority: CompatibilityIdentityAuthority::BitBakeVersionProbe,
            },
            oe_core: CompatibilityDetected::Detected {
                value: CompatibilityReleaseIdentity {
                    name: Some("OE-Core".into()),
                    version: Some("5.2".into()),
                },
                authority: CompatibilityIdentityAuthority::ReleaseMetadata,
            },
            poky: CompatibilityDetected::Detected {
                value: CompatibilityReleaseIdentity {
                    name: Some("wrynose".into()),
                    version: Some("6.0".into()),
                },
                authority: CompatibilityIdentityAuthority::ReleaseMetadata,
            },
            distro: CompatibilityDetected::Detected {
                value: CompatibilityDistroIdentity {
                    name: "poky".into(),
                    version: Some("6.0".into()),
                },
                authority: CompatibilityIdentityAuthority::BitBakeDatastore,
            },
            machine: CompatibilityDetected::Detected {
                value: "qemux86-64".into(),
                authority: CompatibilityIdentityAuthority::BitBakeDatastore,
            },
            layer_series: CompatibilityDetected::Unknown,
            available_tools: CompatibilityDetected::Detected {
                value: vec![CompatibilityToolIdentity {
                    id: "bitbake".into(),
                    executable: "/work/poky/bitbake/bin/bitbake".into(),
                    version: Some("2.18.0".into()),
                }],
                authority: CompatibilityIdentityAuthority::ExecutableProbe,
            },
            backend: CompatibilityDetected::Detected {
                value: CompatibilityBackendIdentity {
                    name: "tinfoil".into(),
                    version: Some("2.18".into()),
                },
                authority: CompatibilityIdentityAuthority::BackendHandshake,
            },
            protocol: CompatibilityDetected::Detected {
                value: CompatibilityProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: "1.0".into(),
                },
                authority: CompatibilityIdentityAuthority::ProtocolNegotiation,
            },
        },
        capabilities: vec![
            CompatibilityCapabilityData {
                id: "bitbake.build".into(),
                state: CompatibilityStateData::Available,
                evidence: vec![evidence(
                    CompatibilityEvidenceKind::DirectProbe,
                    CompatibilityEvidenceOutcome::Positive,
                    "bitbake",
                    "Build command is available.",
                )],
                implementation: Some(CompatibilityImplementationData {
                    id: "bitbake.build.command".into(),
                    kind: "command".into(),
                }),
            },
            CompatibilityCapabilityData {
                id: "bitbake.getvar".into(),
                state: CompatibilityStateData::AvailableWithLimitations {
                    reason: reason(
                        "compatibility.fallback",
                        "Environment dump fallback selected.",
                        "bitbake -e",
                    ),
                    limitations: vec!["Native getvar is absent.".into()],
                },
                evidence: vec![evidence(
                    CompatibilityEvidenceKind::DirectProbe,
                    CompatibilityEvidenceOutcome::Positive,
                    "bitbake",
                    "Environment dump is available.",
                )],
                implementation: Some(CompatibilityImplementationData {
                    id: "bitbake.getvar.environment_fallback".into(),
                    kind: "command".into(),
                }),
            },
            CompatibilityCapabilityData {
                id: "devtool.upgrade".into(),
                state: CompatibilityStateData::Unavailable {
                    reason: reason(
                        "probe.executable_absent",
                        "Devtool is absent from the initialized environment.",
                        "devtool upgrade",
                    ),
                },
                evidence: vec![evidence(
                    CompatibilityEvidenceKind::ExecutableIdentity,
                    CompatibilityEvidenceOutcome::Negative,
                    "devtool",
                    "devtool is absent from the initialized environment",
                )],
                implementation: None,
            },
            CompatibilityCapabilityData {
                id: "resulttool".into(),
                state: CompatibilityStateData::Unknown {
                    reason: reason(
                        "probe.timed_out",
                        "The resulttool probe timed out.",
                        "resulttool --help",
                    ),
                },
                evidence: vec![evidence(
                    CompatibilityEvidenceKind::DirectProbe,
                    CompatibilityEvidenceOutcome::Inconclusive,
                    "resulttool",
                    "Read-only probe timed out.",
                )],
                implementation: None,
            },
            CompatibilityCapabilityData {
                id: "git_archive".into(),
                state: CompatibilityStateData::Unsupported {
                    reason: reason(
                        "yoctui.not_implemented",
                        "No maintained adapter exists.",
                        "oe-git-archive",
                    ),
                },
                evidence: Vec::new(),
                implementation: None,
            },
        ],
    }
}
