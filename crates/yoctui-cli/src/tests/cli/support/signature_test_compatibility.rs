use super::*;

pub(crate) fn signature_test_compatibility(
    build_directory: &Path,
) -> yoctui_model::DaemonCompatibilitySnapshot {
    let capabilities = [
        (
            yoctui_model::CapabilityId::BitBakeDumpSig,
            yoctui_bitbake::BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
        ),
        (
            yoctui_model::CapabilityId::BitBakeDiffSigs,
            yoctui_bitbake::BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
        ),
    ];
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 1,
            environment: yoctui_model::YoctoEnvironmentIdentity {
                build_directory: yoctui_model::AuthoritativeValue::detected(
                    build_directory.to_owned(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: yoctui_model::AuthoritativeValue::detected(
                    ["bitbake-dumpsig", "bitbake-diffsigs"]
                        .into_iter()
                        .map(|name| yoctui_model::ToolIdentity {
                            id: name.into(),
                            executable: build_directory.join(name),
                            version: None,
                        })
                        .collect(),
                    yoctui_model::IdentityAuthority::ExecutableProbe,
                ),
                ..yoctui_model::YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _)| yoctui_model::CapabilityRecord {
                    id: *id,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} test probe", id.as_str()),
                        detail: "Fixture exposes the exact signature helper argv.".into(),
                        argv: Vec::new(),
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|(id, implementation)| {
                (
                    id,
                    yoctui_model::CapabilityImplementation {
                        id: implementation.into(),
                        kind: yoctui_model::CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}
